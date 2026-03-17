use std::{fmt::Debug, sync::Arc};
use actix_service::{IntoServiceFactory, ServiceFactory};
use actix_web::http::Method;
use actix_web::{Resource, Route, Scope};
use actix_web::body::BoxBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::web::{ServiceConfig, scope};

pub trait RouteMounter: Send + Sync + 'static + Debug {
    fn mount(&self, cfg: &mut ServiceConfig);
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeRoute {
    Mounter,
    Resource,
    Route,
    Scope,
}

#[derive(Clone)]
pub enum RouteType {
    Mounter(Arc<dyn RouteMounter>),
    Resource(Arc<dyn Fn() -> Resource + Send + Sync>),
    Route(String, Arc<dyn Fn() -> Route + Send + Sync>),
    Scope(Arc<dyn Fn() -> Scope + Send + Sync>),
}

impl RouteType {
    pub fn into_type_route(&self) -> TypeRoute {
        match self {
            RouteType::Mounter(_) => TypeRoute::Mounter,
            RouteType::Resource(_) => TypeRoute::Resource,
            RouteType::Route(_, _) => TypeRoute::Route,
            RouteType::Scope(_) => TypeRoute::Scope,
        }
    }
    fn dispatch(&self, cfg: &mut ServiceConfig) {
        match self {
            RouteType::Mounter(mounter) => {
                mounter.mount(cfg);
            },
            RouteType::Resource(resource) => {
                cfg.service(resource());
            },
            RouteType::Route(path, route) => {
                cfg.route(&path, route());
            },
            RouteType::Scope(scope) => {
                cfg.service(scope());
            },
        };
    }
}

impl PartialEq for RouteType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RouteType::Mounter(a), RouteType::Mounter(b)) => Arc::ptr_eq(a, b),
            (RouteType::Resource(a), RouteType::Resource(b)) => Arc::ptr_eq(a, b),
            (RouteType::Route(path_a, a), RouteType::Route(path_b, b)) => path_a == path_b && Arc::ptr_eq(a, b),
            (RouteType::Scope(a), RouteType::Scope(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Debug for RouteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteType::Scope(_) => {
                write!(f, "{}", "RouteCollection(Scope)")
            },
            RouteType::Route(s, _) => {
                write!(f, "RouteCollection('{}', Route)", s)
            },
            RouteType::Resource(_) => {
                write!(f, "{}", "RouteCollection(Resource)")
            },
            RouteType::Mounter(_) => {
                write!(f, "{}", "RouteCollection(Mounter)")
            }
        }
    }
}

#[derive(Default)]
pub struct Routes {
    /// A vector of route collections, which can be scopes, routes, resources, or mounters.
    collections: Vec<RouteType>,
    /// A function that takes a mutable reference to `ServiceConfig` and configures the default service.
    default_service: Option<Arc<dyn Fn(&mut ServiceConfig) + Send + Sync>>,
}

pub trait Orchestrator: Debug + Send + Sync + 'static {
    /// Ensemble routes
    fn orchestra(routes: &mut Routes) -> &mut Routes;
}

impl Debug for Routes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RouteCollections {{ collections: {:?}, default_service: {} }}", self.collections, if self.default_service.is_some() { "Some" } else { "None" })
    }
}

impl Routes {
    pub fn default_service<F, U>(&mut self, f: F) -> &mut Self
    where
        F: IntoServiceFactory<U, ServiceRequest> + Clone + Send + Sync + 'static,
        U: ServiceFactory<
                ServiceRequest,
                Config = (),
                Response = ServiceResponse<BoxBody>,
                Error = actix_web::Error,
            > + 'static,
        U::Service: 'static,
        U::InitError: Debug,
    {
        self.default_service = Some(Arc::new(move |cfg| {
            cfg.default_service(f.clone());
        }));

        self
    }

    pub fn add(&mut self, route: RouteType) -> &mut Self
    {
        self.collections.push(route);
        self
    }

    pub fn extend<T: Into<RouteType>>(&mut self, routes: Vec<T>) -> &mut Self
    {
        for route in routes {
            self.collections.push(route.into());
        }
        self
    }

    pub fn dispatch(&self, cfg: &mut ServiceConfig) {
        for route in self.collections.iter() {
            route.dispatch(cfg);
        }
        if let Some(default_service) = &self.default_service {
            default_service(cfg);
        }
    }
}

impl Routes {

    pub fn orchestra<O: Orchestrator>(&mut self) -> &mut Self
    {
        O::orchestra(self);
        self
    }

    pub fn mount<X: RouteMounter + Default>(&mut self) -> &mut Self
    {
        let mounter = Arc::new(X::default());
        self.collections.push(RouteType::Mounter(mounter.clone()));
        self
    }

    pub fn mounter<X: RouteMounter, T: Into<X>>(&mut self, mounter: T) -> &mut Self
    {
        let mounter = Arc::new(mounter.into());
        self.collections.push(RouteType::Mounter(mounter.clone()));
        self
    }

    pub fn resource<F>(&mut self, factory: F) -> &mut Self
        where
            F: Fn() -> Resource + Send + Sync + 'static
    {
        self.collections.push(RouteType::Resource(Arc::new(factory)));
        self
    }

    pub fn scope<F>(&mut self, factory: F) -> &mut Self
    where
        F: Fn() -> Scope + Send + Sync + 'static
    {
        self.collections.push(RouteType::Scope(Arc::new(factory)));
        self
    }

    pub fn route<P: Into<String>, F>(&mut self, path: P, factory: F) -> &mut Self
    where
        F: Fn() -> Route + Send + Sync + 'static
    {
        let path = path.into();
        self.collections.push(RouteType::Route(path, Arc::new(factory)));
        self
    }

    pub fn group<F, X, I>(&mut self, path: I, callback: F) -> &mut Self
        where
            I: Into<String>,
            F: Fn(Scope) -> Scope + Send + Sync + 'static
    {
        let path_str = path.into();
        self.scope(move ||callback(scope(&path_str)));
        self
    }
}

impl Routes {
    pub fn method<F, P, T>(
        &mut self,
        path: P,
        method: T,
        callback: F
    ) -> &mut Self
    where
        F: Fn(Route, &str, Method) -> Route + Send + Sync + 'static,
        P: Into<String>,
        T: Into<Method>
    {
        let path = path.into();
        let method = method.into();
        self.route(path.clone(), move || {
            let route = Route::new().method(method.clone());
            callback(route, &path, method.clone())
        });
        self
    }

    pub fn get<F, P>(&mut self, path: P, callback: F) -> &mut Self
    where
        F: Fn(Route, &str) -> Route + Send + Sync + 'static,
        P: Into<String>
    {
        self.method(path, Method::GET, move |route, path, method| {
            callback(route, path)
        })
    }

    pub fn post<F, P>(&mut self, path: P, callback: F) -> &mut Self
    where
        F: Fn(Route, &str) -> Route + Send + Sync + 'static,
        P: Into<String>
    {
        self.method(path, Method::POST, move |route, path, method| {
            callback(route, path)
        })
    }

    pub fn put<F, P>(&mut self, path: P, callback: F) -> &mut Self
    where
        F: Fn(Route, &str) -> Route + Send + Sync + 'static,
        P: Into<String>
    {
        self.method(path, Method::PUT, move |route, path, method| {
            callback(route, path)
        })
    }

    pub fn delete<F, P>(&mut self, path: P, callback: F) -> &mut Self
    where
        F: Fn(Route, &str) -> Route + Send + Sync + 'static,
        P: Into<String>
    {
        self.method(path, Method::DELETE, move |route, path, method| {
            callback(route, path)
        })
    }

    pub fn patch<F, P>(&mut self, path: P, callback: F) -> &mut Self
    where
        F: Fn(Route, &str) -> Route + Send + Sync + 'static,
        P: Into<String>
    {
        self.method(path, Method::PATCH, move |route, path, method| {
            callback(route, path)
        })
    }

    pub fn head<F, P>(&mut self, path: P, callback: F) -> &mut Self
    where
        F: Fn(Route, &str) -> Route + Send + Sync + 'static,
        P: Into<String>
    {
        self.method(path, Method::HEAD, move |route, path, method| {
            callback(route, path)
        })
    }

    pub fn options<F, P>(&mut self, path: P, callback: F) -> &mut Self
    where
        F: Fn(Route, &str) -> Route + Send + Sync + 'static,
        P: Into<String>
    {
        self.method(path, Method::OPTIONS, move |route, path, method| {
            callback(route, path)
        })
    }

    pub fn connect<F, P>(&mut self, path: P, callback: F) -> &mut Self
    where
        F: Fn(Route, &str) -> Route + Send + Sync + 'static,
        P: Into<String>
    {
        self.method(path, Method::CONNECT, move |route, path, method| {
            callback(route, path)
        })
    }

    pub fn trace<F, P>(&mut self, path: P, callback: F) -> &mut Self
    where
        F: Fn(Route, &str) -> Route + Send + Sync + 'static,
        P: Into<String>
    {
        self.method(path, Method::TRACE, move |route, path, method| {
            callback(route, path)
        })
    }

    pub fn any<F, P>(&mut self, path: P, callback: F) -> &mut Self
    where
        F: Fn(Route, &str) -> Route + Send + Sync + 'static,
        P: Into<String>
    {
        let path = path.into();
        self.route(&path.clone(), move || {
            callback(Route::new(), &path)
        })
    }
}
