use tokio::sync::mpsc::{Receiver, Sender};

use crate::{Message, Process, ProcessGuard, ProcessWrapper};

use super::process::ToSystemMessage;

/// Wrapper around user-defined [process][crate::Process],
/// which allows to send and receive local messages from the it.
pub struct IOProcessWrapper<P: Process + 'static> {
    pub(crate) wrapper: ProcessWrapper<P>,
    /// Allows to send local messages to the process.
    pub sender: Sender<Message>,
    /// Allows to receive local messages from the process.
    pub receiver: Receiver<Message>,

    pub(crate) system_sender: Option<Sender<ToSystemMessage>>,
    pub(crate) proc_name: String,
}

impl<P: Process + 'static> IOProcessWrapper<P> {
    /// Allows to stop the process.
    /// This method is returned when process receives stop request.
    /// After this, it is not guaranteed the process will be stopped immediately.
    pub async fn stop_process(&mut self) {
        self.system_sender
            .take()
            .unwrap()
            .send(ToSystemMessage::ProcessStopped(self.proc_name.clone()))
            .await
            .unwrap();
    }

    /// Returns read access guard to user-defined process.
    /// See [ProcessGuard][ProcessGuard] documentation for more details.
    pub fn read(&self) -> ProcessGuard<'_, P> {
        self.wrapper.read()
    }
}
