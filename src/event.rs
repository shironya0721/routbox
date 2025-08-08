#[derive(Debug, Clone)]
pub enum InputEvent {
    KeyPressed(String),
    KeyReleased(String),
}