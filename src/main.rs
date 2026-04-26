mod config;
mod distribution;
mod models;
mod simulate;

fn main() {
    let mut system_state = models::SystemState::new(
        config::constants::RESOURCE,
        config::constants::ROBOT_UNIFORM_MIN,
        config::constants::ROBOT_UNIFORM_MAX,
        config::constants::MACHINE_UNIFORM_MIN,
        config::constants::MACHINE_UNIFORM_MAX,
        config::constants::TRIANGULAR_LEFT,
        config::constants::TRIANGULAR_RIGHT,
        config::constants::SEED,
    );

    // фаза ввода
    let transaction_time = system_state
        .right_triangular_distr
        .sample(&mut system_state.rng);
    let transaction = models::Transaction::new(1, transaction_time);
    system_state.fec.add(transaction);

    simulate::run(&mut system_state);
}
