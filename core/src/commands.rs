use crate::actions::PoolAction;

pub enum PoolCommand {
    BootstrapPool,
    RefreshPool,
}

pub enum PoolCommandError {}

pub fn build_action(cmd: PoolCommand) -> Result<PoolAction, PoolCommandError> {
    todo!()
}
