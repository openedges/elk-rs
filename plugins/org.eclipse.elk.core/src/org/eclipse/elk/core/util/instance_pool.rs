use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

pub trait IFactory<T>: Send + Sync {
    fn create(&self) -> T;
    fn destroy(&self, obj: T);
}

pub struct InstancePool<T> {
    factory: Box<dyn IFactory<T> + Send + Sync>,
    instances: Mutex<Vec<T>>,
    limit: isize,
}

impl<T> InstancePool<T> {
    pub const INFINITE: isize = -1;

    pub fn new(factory: Box<dyn IFactory<T> + Send + Sync>) -> Self {
        InstancePool {
            factory,
            instances: Mutex::new(Vec::new()),
            limit: Self::INFINITE,
        }
    }

    pub fn with_limit(factory: Box<dyn IFactory<T> + Send + Sync>, limit: isize) -> Self {
        InstancePool {
            factory,
            instances: Mutex::new(Vec::new()),
            limit,
        }
    }

    pub fn fetch(&self) -> T {
        let mut instances = self.instances.lock();        if let Some(obj) = instances.pop() {
            return obj;
        }
        drop(instances);
        self.factory.create()
    }

    pub fn release(&self, obj: T) {
        let mut instances = self.instances.lock();        let should_store = self.limit < 0 || (instances.len() as isize) < self.limit;
        if should_store {
            instances.push(obj);
            return;
        }
        drop(instances);
        self.factory.destroy(obj);
    }

    pub fn clear(&self) {
        let mut instances = self.instances.lock();        for obj in instances.drain(..) {
            self.factory.destroy(obj);
        }
    }

    pub fn destroy(&self, obj: T) {
        self.factory.destroy(obj);
    }
}
