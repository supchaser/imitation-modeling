use crate::models;

pub fn run(system_state: &mut crate::models::SystemState) {
    let time = 1000000.0;
    while !system_state.fec.is_empty() || !system_state.cec.is_empty() {
        if system_state.get_current_time() >= time {
            break;
        }

        let Some(transaction) = system_state.fec.peek() else {
            return;
        };

        // фаза коррекции таймера
        if system_state.cec.is_empty() {
            // если CEC пустая, то корректируем время до первой транзакции из FEC
            let new_current_time = system_state.get_current_time() + transaction.get_time();
            system_state.set_current_time(new_current_time);

            // пока FEC не пустая и время транзактов меньше либо равно текущему времени в системе
            while let Some(transaction) = system_state.fec.peek()
                && transaction.get_time() <= system_state.get_current_time()
            {
                // перекидываем все возможные транзакты в CEC
                let transaction = system_state.fec.pop().unwrap();
                system_state.cec.add_to_back(transaction);
            }
        }

        while let Some(transaction) = system_state.cec.pop_front() {
            execute_block(system_state, transaction);
        }
    }

    println!("COUNT OF COMPLETED DETAILS: {}", system_state.get_count_of_completed_details());
}

fn execute_block(sys_state: &mut models::SystemState, mut t: models::Transaction) {
    match t.get_current_block() {
        models::BlockType::Initial => {
            t.set_current_block(models::BlockType::Generate);
            t.set_next_block(models::BlockType::SeizeRobotToMachiningCenter);
            // возвращаем обратно в CEC
            sys_state.cec.add_to_front(t);
            return;
        }
        models::BlockType::Generate => {
            // создаем новый транзакт и помещаем его в FEC
            let id = t.get_id() + 1;
            let time = sys_state.right_triangular_distr.sample(&mut sys_state.rng)
                + sys_state.get_current_time();
            let new_transaction = models::Transaction::new(id, time);
            // добавляем новый транзакт в FEC
            sys_state.fec.add(new_transaction);

            // обновляем текущий транзакт
            t.set_current_block(models::BlockType::SeizeRobotToMachiningCenter);
            t.set_next_block(models::BlockType::AdvanceRobotToMachiningCenter);
            // возвращаем обратно в CEC
            sys_state.cec.add_to_front(t);
            return;
        }
        models::BlockType::SeizeRobotToMachiningCenter => {
            t.set_current_block(models::BlockType::AdvanceRobotToMachiningCenter);
            t.set_next_block(models::BlockType::ReleaseRobotToMachiningCenter);
            // добавляем транзакт в очередь на робота
            sys_state.add_to_robot_queue(t);

            // возвращаем обратно в CEC
            sys_state.cec.add_to_front(t);
            return;
        }
        models::BlockType::AdvanceRobotToMachiningCenter => {
            // робот занят, заготовка остается в очереди на робота и перемещатеся в FEC (нужна ФКТ)
            if sys_state.robot_is_busy() {
                sys_state.fec.add(t);
                return;
            }

            // робот свободен => занимаем его
            let mut advance_time = sys_state.robot_uniform_distr.sample(&mut sys_state.rng);
            if t.get_id() == 3 {
                advance_time = 1000.0;
            }
            t.set_time(t.get_time() + advance_time);
            // робот занят, пока транзакт его не освободит
            sys_state.set_robot_busy_until(t.get_time());
            t.set_current_block(models::BlockType::ReleaseRobotToMachiningCenter);
            t.set_next_block(models::BlockType::EnterMachiningCenter);
            sys_state.fec.add(t);
            return;
        }
        models::BlockType::ReleaseRobotToMachiningCenter => {
            // удаляем из очереди первую заготовку
            sys_state.delete_from_robot_queue();

            t.set_current_block(models::BlockType::EnterMachiningCenter);
            t.set_next_block(models::BlockType::AdvanceMachiningCenter);
            sys_state.cec.add_to_front(t);

            // добавляем в очередь на обрабатывающий центр
            sys_state.add_to_machines_queue(t);
            return;
        }
        models::BlockType::EnterMachiningCenter => {
            // если все возможные обрабатывающие центры заняты, то оставляем деталь в очереди
            if sys_state.get_machines_busy_count() == sys_state.get_resource() {
                // нужна ФКТ
                sys_state.fec.add(t);
                return;
            }

            // занимаем обрабатывающий центр
            let new_machines_busy_count = sys_state.get_machines_busy_count() + 1;

            sys_state.set_machines_busy_count(new_machines_busy_count);
            t.set_current_block(models::BlockType::AdvanceMachiningCenter);
            t.set_next_block(models::BlockType::LeaveMachiningCenter);
            sys_state.cec.add_to_front(t);
            return;
        }
        models::BlockType::AdvanceMachiningCenter => {
            let advance_time = sys_state.machine_uniform_distr.sample(&mut sys_state.rng);
            t.set_current_block(models::BlockType::LeaveMachiningCenter);
            t.set_next_block(models::BlockType::SeizeRobotToConveyor);
            t.set_time(t.get_time() + advance_time);
            sys_state.fec.add(t);
            return;
        }
        models::BlockType::LeaveMachiningCenter => {
            // робот занят, оставляем деталь в очереди на робота
            if sys_state.robot_is_busy() {
                // нужна ФКТ
                sys_state.fec.add(t);
                return;
            }

            // освобождаем обрабатывающий центр
            let new_machines_busy_count = sys_state.get_machines_busy_count() - 1;

            sys_state.set_machines_busy_count(new_machines_busy_count);
            sys_state.delete_from_machines_queue();

            t.set_current_block(models::BlockType::SeizeRobotToConveyor);
            t.set_next_block(models::BlockType::AdvanceRobotToConveyor);
            sys_state.cec.add_to_front(t);

            return;
        }
        models::BlockType::SeizeRobotToConveyor => {
            t.set_current_block(models::BlockType::AdvanceRobotToConveyor);
            t.set_next_block(models::BlockType::ReleaseRobotToConveyor);
            sys_state.add_to_robot_queue(t);

            sys_state.cec.add_to_front(t);
            return;
        }
        models::BlockType::AdvanceRobotToConveyor => {
            // робот занят, заготовка остается в очереди на робота
            if sys_state.robot_is_busy() {
                sys_state.fec.add(t);
                return;
            }

            // робот свободен
            let advance_time = sys_state.robot_uniform_distr.sample(&mut sys_state.rng);
            t.set_time(t.get_time() + advance_time);
            // робот занят, пока транзакт его не освободит
            sys_state.set_robot_busy_until(t.get_time());

            t.set_current_block(models::BlockType::ReleaseRobotToConveyor);
            t.set_next_block(models::BlockType::Terminate);
            sys_state.fec.add(t);
            return;
        }
        models::BlockType::ReleaseRobotToConveyor => {

            // удаляем из очереди первую деталь
            sys_state.delete_from_robot_queue();

            t.set_current_block(models::BlockType::Terminate);
            t.set_next_block(models::BlockType::Terminate);
            sys_state.cec.add_to_front(t);
            return;
        }
        models::BlockType::Terminate => {
            sys_state.inc_count_of_completed_details();
        }
    }
}
