use dsbuild::{Address, Context, Message, Process};
use log::info;

#[derive(Clone, Default)]
pub struct Client {}

impl Process for Client {
    /// Called when process starts interaction with system.
    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
        info!("Process got local message {:?}", msg);

        ctx.send_local(msg);

        let ctx_2 = ctx.clone();

        ctx.spawn(async move {
            ctx_2.sleep(2.0).await;

            ctx_2.set_timer("timer", 3.0);

            info!("Process set timer on 3 seconds.");

            ctx_2.stop();
        });

        Ok(())
    }

    /// Called when previously set timer is fired.
    fn on_timer(&mut self, name: String, ctx: Context) -> Result<(), String> {
        info!("Process got timer {:?}", name);

        Ok(())
    }

    /// Called when process receives message.
    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) -> Result<(), String> {
        info!("Process got network message {:?}", msg);

        Ok(())
    }
}
