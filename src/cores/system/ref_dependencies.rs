use std::{any::{Any, TypeId}, collections::HashMap, rc::Rc};

#[derive(Debug, Default)]
pub struct RefDependencies {
    dependencies: HashMap<TypeId, Rc< dyn Any + Send + Sync + 'static>>
}

impl RefDependencies {
    pub fn register<T: Send + Sync + Default + 'static>(&mut self) -> Option<&T> {
        let id = TypeId::of::<T>();
        let e = self.dependencies.entry(id).or_insert(Rc::new(T::default()));
        e.downcast_ref::<T>()
    }

    pub fn deregister<T: Any + Send + Sync + 'static>(&mut self) -> Option<T> {
        let any_rc = self.dependencies.remove(&TypeId::of::<T>())?;
        if (*any_rc).type_id() != TypeId::of::<T>() {
            return None;
        }
        let raw = Rc::into_raw(any_rc) as *const T;
        let rc_t = unsafe {
            Rc::from_raw(raw)
        };
        Rc::try_unwrap(rc_t).ok()
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.dependencies.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    pub fn get_mut<T: Send + Sync + 'static>(&self) -> Option<&mut T> {
        let rc = self.dependencies.get(&TypeId::of::<T>())?;
        unsafe {
            let const_ptr = Rc::as_ptr(rc) as *const T;
            let mut_ptr = const_ptr as *mut T;
            // Balekno dadi referensi mutabel
            Some(&mut *mut_ptr)
        }
    }

    pub fn replace<T: Send + Sync + Default + 'static>(&mut self) -> Option<&T> {
        let id = TypeId::of::<T>();
        self.dependencies.insert(id, Rc::new(T::default()));
        self.get::<T>()
    }
}
