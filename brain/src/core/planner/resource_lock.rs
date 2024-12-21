//no HashSet to avoid Hash and Eq constraints. Performance should be good enough as not many
//entries are expected
pub struct ResourceLock<R> {
    resources: Vec<R>,
}

pub trait Lockable<R: PartialEq> {
    fn locking_key(&self) -> R;
}

impl<R> ResourceLock<R>
where
    R: PartialEq,
{
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
        }
    }

    pub fn lock(&mut self, resource: &impl Lockable<R>) {
        self.resources.push(resource.locking_key());
    }

    pub fn is_locked(&self, resource: &impl Lockable<R>) -> bool {
        self.resources.contains(&resource.locking_key())
    }
}
