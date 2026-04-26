use crate::models;

pub fn run(system_state: &mut crate::models::SystemState) {
    let time = 100000.0;
    let mut i = 0;
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

            // // пока FEC не пустая и время транзактов меньше либо равно текущему времени в системе
            // while let Some(transaction) = system_state.fec.pop()
            //     && transaction.get_time() <= system_state.get_current_time()
            // {
            //     // перекидываем все возможные транзакты в CEC
            //     system_state.cec.add_to_back(transaction);
            // }
        }

        println!("ITER №: {}, CURRENT_TIME: {}", i, system_state.current_time);
        println!("MACHINES BUSY COUNT: {}", system_state.machines_busy_count);

        // обрабатываем транзакции из fec, переносим их в cec, пока позволяет таймер
        while let Some(transaction) = system_state.fec.pop() {
            // нужно корректировать таймер, возвращаем транзакт в FEC
            if transaction.get_time() > system_state.get_current_time() {
                system_state.fec.add(transaction);
                break;
            }

            // ФП
            println!("EXECUTE FEC TRANSACTION: {:?}", transaction);
            execute_block(system_state, transaction);
        }

        // println!(
        //     "FEC: {:?}, CEC: {:?}",
        //     system_state.fec,
        //     system_state.cec
        // );

        while let Some(transaction) = system_state.cec.delete() {
            println!("EXECUTE CEC TRANSACTION: {:?}", transaction);
            // ФП
            execute_block(system_state, transaction);
        }

        // println!(
        //     "FEC: {:?}, CEC: {:?}",
        //     system_state.fec,
        //     system_state.cec
        // );
        i += 1;
    }

    println!(
        "\nSimulation finished at time: {:.2}",
        system_state.current_time
    );
}

fn execute_block(sys_state: &mut models::SystemState, mut t: models::Transaction) {
    match t.get_current_block() {
        models::BlockType::Initial => {
            t.set_current_block(models::BlockType::Generate);
            t.set_next_block(models::BlockType::SeizeRobotToMachiningCenter);
            return;
        }
        models::BlockType::Generate => {
            // создаем новый транзакт и помещаем его в fec
            let id = t.get_id() + 1;
            let time = sys_state.right_triangular_distr.sample(&mut sys_state.rng)
                + sys_state.get_current_time();
            let new_transaction = models::Transaction::new(id, time);
            sys_state.fec.add(new_transaction);

            // обновляем текущий транзакт
            t.set_current_block(models::BlockType::SeizeRobotToMachiningCenter);
            t.set_next_block(models::BlockType::AdvanceRobotToMachiningCenter);
            sys_state.cec.add_to_front(t);
            return;
        }
        models::BlockType::SeizeRobotToMachiningCenter => {
            t.set_current_block(models::BlockType::AdvanceRobotToMachiningCenter);
            t.set_next_block(models::BlockType::ReleaseRobotToMachiningCenter);
            sys_state.add_to_robot_queue(t);
            sys_state.cec.add_to_front(t);
            return;
        }
        models::BlockType::AdvanceRobotToMachiningCenter => {
            // робот занят, заготовка остается в очереди на робота
            if sys_state.robot_is_busy() {
                sys_state.fec.add(t);
                return;
            }

            // робот свободен
            let advance_time = sys_state.robot_uniform_distr.sample(&mut sys_state.rng);
            t.set_time(t.get_time() + advance_time);
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
            sys_state.cec.add_to_back(t);

            // добавляем в очередь на обрабатывающий центр
            sys_state.add_to_machines_queue(t);
            return;
        }
        models::BlockType::EnterMachiningCenter => {
            // если все возможные обрабатывающие центры заняты, то оставляем деталь в очереди
            if sys_state.get_machines_busy_count() == sys_state.get_resource() {
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
            // if t.get_id() == 2 || t.get_id() == 3 {
            //     advance_time = 1000.0;
            // }
            t.set_current_block(models::BlockType::LeaveMachiningCenter);
            t.set_next_block(models::BlockType::SeizeRobotToConveyor);
            t.set_time(t.get_time() + advance_time);
            if t.get_id() == 2 {
                println!("TIME {}", t.get_time());
            }
            sys_state.fec.add(t);
            return;
        }
        models::BlockType::LeaveMachiningCenter => {
            // робот занят, оставляем деталь в очереди на робота
            if sys_state.robot_is_busy() {
                sys_state.fec.add(t);
                return;
            }

            if t.get_id() == 2 {
                println!("TTTTTTTUUUUUUUUUUTTTTTTTT");
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
            t.set_current_block(models::BlockType::ReleaseRobotToConveyor);
            t.set_next_block(models::BlockType::Terminate);
            sys_state.fec.add(t);
            return;
        }
        models::BlockType::ReleaseRobotToConveyor => {
            if sys_state.get_current_time() < t.get_time() {
                return;
            }

            // удаляем из очереди первую деталь
            sys_state.delete_from_robot_queue();

            t.set_current_block(models::BlockType::Terminate);
            t.set_next_block(models::BlockType::Terminate);
            sys_state.cec.add_to_back(t);
            return;
        }
        models::BlockType::Terminate => {
            sys_state.cec.delete();
        }
    }
}
