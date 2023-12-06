


#[derive(Default)]
pub enum UpdateStatus {
    #[default]
    Unchecked,
    Checking,
    Outdated,
    Updated
}