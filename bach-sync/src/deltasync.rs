pub type CB<T> = Box<dyn Fn(T) + Send + Sync + 'static>;

pub struct Callable<T> {
    f: CB<T>,
}

impl<T> Callable<T> {
    pub fn new(f: CB<T>) -> Self {
        Callable { f }
    }

    pub fn run(&self, p: T) {
        (self.f)(p)
    }
}

impl<T> std::ops::Deref for Callable<T>
where
    T: 'static,
{
    type Target = dyn Fn(T);

    fn deref(&self) -> &Self::Target {
        &self.f
    }
}

impl<T> std::ops::DerefMut for Callable<T>
where
    T: 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.f
    }
}

pub mod fssync;
pub mod sshsync;
