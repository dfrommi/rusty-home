//no HashSet to avoid Hash and Eq constraints. Performance should be good enough as not many
//entries are expected
pub struct ResourceLock<R> {
    resources: Vec<R>,
}

impl<R> ResourceLock<R>
where
    R: PartialEq,
{
    pub fn new() -> Self {
        Self { resources: Vec::new() }
    }

    pub fn lock(&mut self, resource: R) {
        self.resources.push(resource);
    }

    pub fn is_locked(&self, resource: &R) -> bool {
        self.resources.contains(resource)
    }
}
