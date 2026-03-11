pub trait Snapshot: Send + Sync + Sized {
    fn get_snapshot(&self) -> Option<Self>;
    fn has_snapshot(&self) -> bool where {
        self.get_snapshot().is_some()
    }
}
