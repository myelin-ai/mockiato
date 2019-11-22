use mockiato::mockable;
use std::error::Error;

#[mockable]
trait MessageSender {
    fn send_message(&self, message: &str) -> Result<(), Box<dyn Error>>;
}

#[test]
fn return_value_with_fn() {
    let mut message_sender = MessageSenderMock::new();
    message_sender
        .expect_send_message(|arg| arg.partial_eq("foo"))
        .returns_with(|_| Ok(()));
    assert!(message_sender.send_message("foo").is_ok());
}
