use crate::models::{self, SystemState};
use std::fs::File;
use crate::print;

pub fn run(system_state: &mut crate::models::SystemState) {
    let mut file = File::create("output.csv").expect("Не удалось создать файл");
    print::write_header(&mut file).unwrap();

    print::write_log(
        &mut file,
        "ФВ",
        system_state.get_current_time(),
        system_state.get_machines_busy_count(),
        &print::format_fec(&system_state.fec),
        &print::format_cec(&system_state.cec),
    ).unwrap();

    let time = 10000.0;
    while !system_state.fec.is_empty() || !system_state.cec.is_empty() {
    // for n in 0..103 {
        // println!("\\\\\\\\\\ ITER # {} \\\\\\\\\\\\", n);
        if system_state.get_current_time() >= time {
            break;
        }

        // фаза коррекции таймера
        correction_time(system_state, &mut file);

        // println!("FEC AFTER CORRECTION TIME: {:?}", system_state.fec);
        // println!("CEC AFTER CORRECTION TIME: {:?}", system_state.cec);
        // println!(
        //     "-----------------------------------------------------------------------------------"
        // );

        // println!(
        //         "MACHINES BUSY CURRENT: {}",
        //         system_state.get_machines_busy_count()
        // );


        // println!("CURRENT_TIME: {}", system_state.get_current_time());

        // фаза просмотра
        while !system_state.cec.is_empty() {
            print::write_log(
                &mut file,
                "ФП",
                system_state.get_current_time(),
                system_state.get_machines_busy_count(),
                &print::format_fec(&system_state.fec),
                &print::format_cec(&system_state.cec),
            ).unwrap();

            let transaction = system_state.cec.pop_front().unwrap();
            execute_block(system_state, transaction);
        }
    }

    system_state.update_machines_queue_stats();

    let total_time = system_state.get_current_time();
    let robot_load = system_state.total_robot_busy_time / total_time;
    let machines_load =
        system_state.total_machines_busy_time / (system_state.resource as f64 * total_time);
    let avg_queue_len = system_state.total_queue_length_time / total_time;
    println!("TOTAL_QUEUE_LENGTH_TIME: {}", system_state.total_queue_length_time);
    let avg_wait = if system_state.total_robot_wait_count > 0 {
        system_state.total_robot_wait_time / system_state.total_robot_wait_count as f64
    } else {
        0.0
    };

    println!("total_robot_wait_time: {}", system_state.total_robot_wait_time);

    println!("Общее время работы системы: {:.2}", total_time);
    println!("Коэффициент загрузки робота: {:.4}", robot_load);
    println!("Коэффициент загрузки станков: {:.4}", machines_load);
    println!("Средняя длина очереди заготовок: {:.4}", avg_queue_len);
    println!("Среднее время ожидания в очереди: {:.4}", avg_wait);
    println!(
        "Число обработанных деталей: {}",
        system_state.get_count_of_completed_details()
    );
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

            t.robot_wait_start = sys_state.get_current_time();

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

            let wait = sys_state.get_current_time() - t.robot_wait_start;
            sys_state.total_robot_wait_time += wait;
            sys_state.total_robot_wait_count += 1;

            // удаляем из очереди первую заготовку
            sys_state.delete_from_robot_queue();

            // робот свободен => занимаем его
            let advance_time = sys_state.robot_uniform_distr.sample(&mut sys_state.rng);
            t.set_time(t.get_time() + advance_time);
            sys_state.total_robot_busy_time += advance_time;
            // робот занят, пока транзакт его не освободит
            sys_state.set_robot_busy_until(t.get_time());
            t.set_current_block(models::BlockType::ReleaseRobotToMachiningCenter);
            t.set_next_block(models::BlockType::EnterMachiningCenter);
            sys_state.fec.add(t);
            return;
        }
        models::BlockType::ReleaseRobotToMachiningCenter => {
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
                println!("TTTTTTUUUUUUUUUTTTTTTTAAAAA");
                // нужна ФКТ
                sys_state.fec.add(t);
                return;
            }

            // занимаем обрабатывающий центр
            let new_machines_busy_count = sys_state.get_machines_busy_count() + 1;
            sys_state.delete_from_machines_queue();
            sys_state.set_machines_busy_count(new_machines_busy_count);
            t.set_current_block(models::BlockType::AdvanceMachiningCenter);
            t.set_next_block(models::BlockType::LeaveMachiningCenter);
            sys_state.cec.add_to_front(t);
            return;
        }
        models::BlockType::AdvanceMachiningCenter => {
            let advance_time = sys_state.machine_uniform_distr.sample(&mut sys_state.rng);
            sys_state.total_machines_busy_time += advance_time;
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

            t.set_current_block(models::BlockType::SeizeRobotToConveyor);
            t.set_next_block(models::BlockType::AdvanceRobotToConveyor);
            sys_state.cec.add_to_front(t);

            return;
        }
        models::BlockType::SeizeRobotToConveyor => {
            t.set_current_block(models::BlockType::AdvanceRobotToConveyor);
            t.set_next_block(models::BlockType::ReleaseRobotToConveyor);
            sys_state.add_to_robot_queue(t);

            t.robot_wait_start = sys_state.get_current_time();

            sys_state.cec.add_to_front(t);
            return;
        }
        models::BlockType::AdvanceRobotToConveyor => {
            // робот занят, заготовка остается в очереди на робота
            if sys_state.robot_is_busy() {
                sys_state.fec.add(t);
                return;
            }

            let wait = sys_state.get_current_time() - t.robot_wait_start;
            sys_state.total_robot_wait_time += wait;
            sys_state.total_robot_wait_count += 1;

            // робот свободен
            let advance_time = sys_state.robot_uniform_distr.sample(&mut sys_state.rng);
            t.set_time(t.get_time() + advance_time);
            sys_state.total_robot_busy_time += advance_time;
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

fn correction_time(sys_state:&mut SystemState, file: &mut std::fs::File) {
        let Some(transaction) = sys_state.fec.peek() else {
            return;
        };
            // фаза коррекции таймера
        if sys_state.cec.is_empty() {
            // если CEC пустая, то корректируем время до первой транзакции из FEC
            let mut new_current_time = transaction.get_time();
            if new_current_time < sys_state.robot_busy_until {
                new_current_time = sys_state.robot_busy_until;
            }

            // если оба обрабатывающих центра заняты и текущий транзакт стоит в очереди, то сначала освободим обрабатывающий центр
            if sys_state.get_machines_busy_count() == sys_state.get_resource() && 
                transaction.current_block == models::BlockType::EnterMachiningCenter
                {
                    let mut temp_vec = Vec::new();
                    let mut found = None;
                    // ищем транзакцию, которая хочет покинуть обрабатывающий центр, корректируем время
                    while let Some(transaction) = sys_state.fec.peek() {
                        if transaction.get_current_block() == models::BlockType::LeaveMachiningCenter {
                            found = Some(transaction.get_time());
                            break;
                        }

                        temp_vec.push(sys_state.fec.pop().unwrap());
                    }

                    for transaction in temp_vec {
                        sys_state.fec.add(transaction);
                    }

                    if let Some(time) = found {
                        new_current_time = time;
                    }
                }

            sys_state.set_current_time(new_current_time);

            // пока FEC не пустая и время транзактов меньше либо равно текущему времени в системе
            while let Some(transaction) = sys_state.fec.peek()
                && transaction.get_time() <= sys_state.get_current_time()
            {
                print::write_log(
                file,
                    "ФКТ",
                    sys_state.get_current_time(),
                    sys_state.get_machines_busy_count(),
                    &print::format_fec(&sys_state.fec),
                    &print::format_cec(&sys_state.cec),
                ).unwrap();
                // перекидываем все возможные транзакты в CEC
                let transaction = sys_state.fec.pop().unwrap();
                sys_state.cec.add_to_back(transaction);
            }
        }
}