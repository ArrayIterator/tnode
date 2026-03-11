use actix_web::Error;
use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::future::{Ready, ready};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll};
use actix_service::boxed::BoxFuture;

pub type MiddlewareResult = BoxFuture<Result<ServiceResponse<BoxBody>, Error>>;
pub type NextFn = Box<dyn FnOnce(ServiceRequest) -> MiddlewareResult>;

pub trait Middleware: Send + Sync + Debug + 'static {
    fn as_any(&self) -> &dyn Any;
    fn get_priority(&self) -> isize {
        0
    }
    fn handle(&self, req: ServiceRequest, next: NextFn) -> MiddlewareResult;
}

impl PartialEq for dyn Middleware {
    fn eq(&self, other: &Self) -> bool {
        self.as_any().type_id() == other.as_any().type_id()
    }
}

#[derive(Debug, Clone)]
pub struct DispatcherMiddleware<S> {
    middlewares: Rc<Vec<Arc<dyn Middleware>>>,
    inner: Rc<S>,
}

#[derive(Debug, Clone)]
pub struct Dispatcher(Rc<Vec<Arc<dyn Middleware>>>);

impl Dispatcher {
    pub fn new(middlewares: Vec<Arc<dyn Middleware>>) -> Self {
        Dispatcher(Rc::new(middlewares))
    }
}

pub struct DispatcherFuture<F> {
    fut: F,
}

impl<F, B> Future for DispatcherFuture<F>
where
    F: Future<Output = Result<ServiceResponse<B>, Error>>,
    B: MessageBody + 'static,
{
    type Output = Result<ServiceResponse, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        match unsafe { Pin::new_unchecked(&mut this.fut) }.poll(cx) {
            Poll::Ready(Ok(res)) => Poll::Ready(Ok(res.map_into_boxed_body())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<S, B> Service<ServiceRequest> for DispatcherMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    // type Future = DispatcherFuture<S::Future>;
    type Future = DispatcherFuture<MiddlewareResult>;
    forward_ready!(inner);
    fn call(&self, req: ServiceRequest) -> Self::Future {
        DispatcherFuture {
            fut: Self::do_dispatch(self.middlewares.clone(), self.inner.clone(), req, 0),
        }
    }
}

impl<S, B> DispatcherMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    fn do_dispatch(
        middlewares: Rc<Vec<Arc<dyn Middleware>>>,
        inner: Rc<S>,
        req: ServiceRequest,
        index: usize,
    ) -> MiddlewareResult {
        if index >= middlewares.len() {
            let fut = inner.call(req);
            return Box::pin(async move { fut.await.map(|res| res.map_into_boxed_body()) });
        }
        let middleware = middlewares[index].clone();
        let mws_clone = middlewares.clone();
        let inner_clone = inner.clone();
        let next: NextFn =
            Box::new(move |r| Self::do_dispatch(mws_clone, inner_clone, r, index + 1));
        middleware.handle(req, next)
    }
}

impl<S, B> Transform<S, ServiceRequest> for Dispatcher
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Transform = DispatcherMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let mut mws = (*self.0).clone();
        mws.sort_by_key(|mw| std::cmp::Reverse(mw.get_priority()));

        ready(Ok(DispatcherMiddleware {
            middlewares: Rc::new(mws),
            inner: Rc::new(service),
        }))
    }
}

/// Manages a collection of middleware in a pipeline.
///
/// Middleware can be appended, prepended, or filtered from the pipeline.
#[derive(Debug, Clone)]
pub struct MiddlewareManager {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareManager {
    /// Creates a new empty middleware manager.
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn has<M: Middleware>(&self) -> bool {
        let type_id = TypeId::of::<M>();
        self.middlewares
            .iter()
            .any(|e| e.as_any().type_id() == type_id)
    }

    pub fn register<M: Middleware + Default>(&mut self) -> &mut Self {
        if self.has::<M>() {
            return self;
        }
        self.middlewares.push(Arc::new(M::default()));
        self
    }

    pub fn remove<M: Middleware>(&mut self) {
        let type_id = TypeId::of::<M>();
        self.middlewares.retain(|e| e.as_any().type_id() != type_id);
    }

    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    pub fn clear(&mut self) {
        self.middlewares.clear();
    }

    pub fn create_dispatcher(&self) -> Dispatcher {
        Dispatcher::new(self.middlewares.clone())
    }
}

impl Default for MiddlewareManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for MiddlewareManager {
    type Item = Arc<dyn Middleware>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.middlewares.into_iter()
    }
}
