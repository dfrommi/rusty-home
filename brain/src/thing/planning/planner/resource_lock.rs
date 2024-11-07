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
        Self {
            resources: Vec::new(),
        }
    }

    pub fn lock(&mut self, resource: Option<R>) {
        if let Some(resource) = resource {
            self.resources.push(resource);
        }
    }

    pub fn is_locked(&self, resource: &Option<R>) -> bool {
        resource
            .as_ref()
            .map_or(false, |resource| self.resources.contains(resource))
    }
}
