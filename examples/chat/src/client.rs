use dsbuild::{Address, Context, Message, Process};

#[derive(Clone)]
pub struct Client {
    pub other: Address,
}

impl Process for Client {
    /// Called when process starts interaction with system.
    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
        let other = self.other.clone();
        ctx.clone().spawn(async move {
            ctx.send_reliable(msg, other).await.unwrap();
        });

        Ok(())
    }

    /// Called when previously set timer is fired.
    fn on_timer(&mut self, name: String, ctx: Context) -> Result<(), String> {
        Ok(())
    }

    /// Called when process receives message.
    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) -> Result<(), String> {
        ctx.send_local(msg);

        ctx.stop();

        Ok(())
    }
}
