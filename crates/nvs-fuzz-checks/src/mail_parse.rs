use nvs_stdlib::fuzz;

/// `Core\Mail\Message::parse` must not panic on any bytes, and the instance it builds must
/// hold the types its class states in every slot. A message never throws: whatever the input,
/// the parse gives a message, so the check reads every input whole.
pub fn mail_parse(data: &[u8]) {
    let message = fuzz::mail_parse(data);
    if let Err(why) = fuzz::check_fields(message.get()) {
        panic!("a slot does not fit its type: {why}");
    }
}
