use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};
use rand::SeedableRng;
use rand::rngs::StdRng;
use crate::distribution;

// BlockType - названия блоков
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BlockType {
    Initial = 0,
    Generate = 1,
    SeizeRobotToMachiningCenter = 2,
    AdvanceRobotToMachiningCenter = 3,
    ReleaseRobotToMachiningCenter = 4,
    EnterMachiningCenter = 5,
    AdvanceMachiningCenter = 6,
    LeaveMachiningCenter = 7,
    SeizeRobotToConveyor = 8,
    AdvanceRobotToConveyor = 9,
    ReleaseRobotToConveyor = 10,
    Terminate = 11,
}

// Transaction - модель транзакта
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Transaction {
    // id - айди транзакта
    pub id: i64,
    // time - время наступления события по таймеру
    pub time: f64,
    // current_block - адрес текущего блока
    pub current_block: BlockType,
    // next_block - адрес следующего блока
    pub next_block: BlockType,
    pub robot_wait_start: f64, 
}

impl Eq for Transaction {}

impl PartialOrd for Transaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Transaction {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .time
            .partial_cmp(&self.time)
            .unwrap_or(Ordering::Equal)
    }
}

// FEC - feature event chain
#[derive(Debug)]
pub struct FEC {
    pub heap: BinaryHeap<Transaction>,
}

// CEC - current event chain
#[derive(Debug)]
pub struct CEC {
    pub queue: VecDeque<Transaction>,
}

// SystemState - общее состояние системы
pub struct SystemState {
    // current_time - текущее время в системе
    current_time: f64,
    // resource - сколько всего обрабатывающих центров
    pub resource: usize,

    // machines_queue - очередь на обрабатывающие центры
    pub machines_queue: VecDeque<Transaction>,
    // machines_busy_count - сколько обрабатывающих центров
    pub machines_busy_count: usize,

    pub fec: FEC,
    pub cec: CEC,

    // robot_busy_until - время до которого занят робот
    pub robot_busy_until: f64,
    // robot_queue - очередь транзактов на робота
    pub robot_queue: VecDeque<Transaction>,

    pub robot_uniform_distr: distribution::UniformDistr,
    pub machine_uniform_distr: distribution::UniformDistr,
    pub right_triangular_distr: distribution::RightTriangular,
    pub rng: StdRng,

    // число заверешенных транзактов
    count_of_completed_details: i64,

    pub total_robot_busy_time: f64,
    pub total_machines_busy_time: f64,
    pub total_robot_wait_time: f64,
    pub total_robot_wait_count: u64,
    pub total_queue_length_time: f64,
    pub last_queue_change_time: f64,
}

impl Transaction {
    pub fn new(id: i64, time: f64) -> Self {
        return Transaction {
            id,
            time,
            current_block: BlockType::Initial,
            next_block: BlockType::Generate,
            robot_wait_start: 0.0,
        };
    }

    pub fn get_id(&self) -> i64 {
        return self.id;
    }

    pub fn get_time(&self) -> f64 {
        return self.time;
    }

    pub fn get_current_block(&self) -> BlockType {
        return self.current_block;
    }

    pub fn set_time(&mut self, new_time: f64) {
        self.time = new_time;
    }

    pub fn set_current_block(&mut self, block: BlockType) {
        self.current_block = block;
    }

    pub fn set_next_block(&mut self, block: BlockType) {
        self.next_block = block;
    }
}

impl FEC {
    fn new() -> Self {
        return FEC {
            heap: BinaryHeap::new(),
        };
    }

    pub fn add(&mut self, transaction: Transaction) {
        self.heap.push(transaction);
    }

    pub fn pop(&mut self) -> Option<Transaction> {
        self.heap.pop()
    }

    pub fn peek(&self) -> Option<&Transaction> {
        self.heap.peek()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}

impl CEC {
    pub fn new() -> Self {
        return CEC {
            queue: VecDeque::new(),
        };
    }

    pub fn add_to_back(&mut self, t: Transaction) {
        self.queue.push_back(t);
    }

    pub fn add_to_front(&mut self, t: Transaction) {
        self.queue.push_front(t);
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn pop_front(&mut self) -> Option<Transaction> {
        let elem = self.queue.pop_front();
        return elem;
    }
}

impl SystemState {
    pub fn new(
        resource: usize, 
        r_min: f64, 
        r_max: f64, 
        m_min: f64,
        m_max: f64,
        left: f64, 
        right: f64, 
        seed: u64,
    ) -> Self {
        return SystemState {
            current_time: 0.0,
            resource: resource,
            machines_busy_count: 0,
            machines_queue: VecDeque::new(),
            fec: FEC::new(),
            cec: CEC::new(),
            robot_busy_until: 0.0,
            robot_queue: VecDeque::new(),
            robot_uniform_distr: distribution::UniformDistr::new(r_min,r_max),
            machine_uniform_distr: distribution::UniformDistr::new(m_min, m_max),
            right_triangular_distr: distribution::RightTriangular::new(left, right),
            rng: StdRng::seed_from_u64(seed),
            count_of_completed_details: 0,
            total_robot_busy_time: 0.0,
            total_machines_busy_time: 0.0,
            total_robot_wait_time: 0.0,
            total_robot_wait_count: 0,
            total_queue_length_time: 0.0,
            last_queue_change_time: 0.0,
        };
    }

    pub fn robot_is_busy(&self) -> bool {
        return self.current_time < self.robot_busy_until
    }

    pub fn inc_count_of_completed_details(&mut self) {
        self.count_of_completed_details = self.count_of_completed_details + 1;
    }

    pub fn get_count_of_completed_details(&self) -> i64 {
        return self.count_of_completed_details;
    }

    pub fn set_robot_busy_until(&mut self, t: f64) {
        self.robot_busy_until = t;
    }

    pub fn get_resource(&self) -> usize {
        return self.resource;
    }

    pub fn get_machines_busy_count(&self) -> usize {
        return self.machines_busy_count;
    }

    pub fn add_to_robot_queue(&mut self, t: Transaction) {
        self.robot_queue.push_back(t);
    }

    pub fn get_current_time(&self) -> f64 {
        return self.current_time;
    }

    pub fn set_current_time(&mut self, new_current_time: f64) {
        self.current_time = new_current_time;
    }

    pub fn add_to_machines_queue(&mut self, t: Transaction) {
        self.update_machines_queue_stats();
        self.machines_queue.push_back(t);
    }

    pub fn delete_from_robot_queue(&mut self) {
        self.update_machines_queue_stats();
        self.robot_queue.pop_front();
    }

    pub fn set_machines_busy_count(&mut self, count: usize) {
        self.machines_busy_count = count;
    }

    pub fn delete_from_machines_queue(&mut self) {
        self.machines_queue.pop_front();
    }

    pub fn update_machines_queue_stats(&mut self) {
        let duration = self.current_time - self.last_queue_change_time;
        self.total_queue_length_time += duration * self.machines_queue.len() as f64;
        self.last_queue_change_time = self.current_time;
    }
}
