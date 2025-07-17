use solana_program::program_error::ProgramError;
use steel::log;

#[allow(dead_code)]
#[track_caller]
pub fn trace(msg: &str, error: ProgramError) -> ProgramError {
    let caller = std::panic::Location::caller();
    log(format!("{}: {}", msg, caller));
    error
}