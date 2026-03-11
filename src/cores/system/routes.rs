use actix_service::IntoServiceFactory;
use actix_web::body::BoxBody;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::web::ServiceConfig;
use actix_web::{web, Error, Scope};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

pub trait RouteScope: Debug + Send + Sync + Any + 'static {
    fn prefix(&self) -> &str;
    fn wrap(&self, scope: Scope) -> Scope {
        scope
    }
}

#[derive(Clone)]
pub struct Routes {
    routes: HashMap<TypeId, Arc<dyn Route>>,
    scopes: HashMap<TypeId, (Arc<dyn RouteScope>, Arc<Vec<TypeId>>)>,
    // <Route::TypeId, Vec<RouteScope::TypeId>>
    route_scope: HashMap<TypeId, Arc<Vec<TypeId>>>,
    default_service: Option<Arc<dyn Fn(&mut ServiceConfig) + Send + Sync>>,
}

impl Debug for Routes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Routes")
            .field("routes", &self.routes)
            .field("scopes", &self.scopes)
            .field(
                "default_service",
                if self.default_service.is_some() {
                    &"Option<Arc<dyn Fn(&mut ServiceConfig) + Send + Sync>>"
                } else {
                    &"None"
                },
            )
            .field("has_default_service", &self.default_service.is_some())
            .finish()
    }
}

pub trait Orchestrator: Debug + Send + Sync + 'static {
    /// Ensemble routes
    fn ensemble(routes: &mut Routes) -> &mut Routes;
}

pub trait Route: Send + Sync + 'static + Debug + Any {
    fn mount(&self, cfg: &mut ServiceConfig, prefix: &str);
}

impl Routes {
    /// Creates a new instance of the structure with initialized default values.
    ///
    /// # Returns
    ///
    /// A new instance of the struct where `routes` is initialized as an empty `Vec`.
    ///
    /// # Example
    /// ```
    /// let instance = StructName::new();
    /// assert!(instance.routes.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            scopes: HashMap::new(),
            route_scope: HashMap::new(),
            default_service: None,
        }
    }

    /// Removes the first route of type `Route` from the internal collection of routes.
    ///
    /// This function searches through the `routes` collection to find the first occurrence of a route
    /// whose type matches the `Route` trait. If such a route is found, it is removed from the collection
    /// and returned as an `Option` containing an `Arc<dyn Route>`. If no matching route is found, `None`
    /// is returned.
    ///
    /// # Returns
    /// - `Some(Arc<dyn Route>)` if a route of type `Route` is found and removed.
    /// - `None` if no matching route is present in the collection.
    ///
    /// # Example
    /// ```
    /// # use your_module::YourStruct; // Replace with the actual module and struct
    /// # use std::sync::Arc;
    /// # trait Route {}
    /// let mut your_instance = YourStruct::new();
    /// // Add some routes to `your_instance`
    ///
    /// if let Some(removed_route) = your_instance.remove() {
    ///     // A route was removed successfully
    ///     println!("Route removed.");
    /// } else {
    ///     // No matching route was found
    ///     println!("No route matched to remove.");
    /// }
    /// ```
    ///
    /// # Notes
    /// - The method uses `TypeId` to check for a match based on the type of the `Route` trait. Ensure
    ///   that the types stored in the collection implement the `Route` trait.
    /// - This method may not be thread-safe if `self.routes` is shared across threads without proper
    ///   synchronization.
    ///
    pub fn remove<T: Route + Default + 'static>(&mut self) -> Option<Arc<dyn Route>> {
        let route_id = TypeId::of::<T>();
        let mut scopes_to_delete = Vec::new();
        if let Some(scope_ids_arc) = self.route_scope.remove(&route_id) {
            for scope_id in scope_ids_arc.iter() {
                if let Some((_, routes_vec_arc)) = self.scopes.get_mut(scope_id) {
                    let mut new_vec: Vec<TypeId> = (**routes_vec_arc).clone();
                    new_vec.retain(|&id| id != route_id);

                    if new_vec.is_empty() {
                        scopes_to_delete.push(*scope_id);
                    } else {
                        *routes_vec_arc = Arc::new(new_vec);
                    }
                }
            }
        }

        for s_id in scopes_to_delete {
            self.scopes.remove(&s_id);
        }

        self.routes.remove(&route_id)
    }

    /// Adds a new route to the current collection of routes.
    ///
    /// # Arguments
    ///
    /// * `route` - An `Arc` containing a trait object of type `Route`.
    ///             This allows for shared ownership and ensures that the
    ///             route can be accessed safely across threads if needed.
    /// * `scope` - A `Option<&str>` web scope path, will trimming slash
    ///
    /// # Behavior
    ///
    /// The provided `route` is pushed into the `routes` vector within the struct.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// struct MyRoute;
    /// impl Route for MyRoute {
    ///     // Implement required methods here.
    /// }
    ///
    /// let mut router = Router::new();
    /// let my_route = Arc::new(MyRoute);
    /// router.add(my_route, None);
    /// ```
    ///
    /// # Note
    ///
    /// Make sure the `Route` trait is implemented for the type of the
    /// object being added as a route.
    ///
    /// # Panics
    ///
    /// This function does not panic under normal circumstances.
    pub fn add<T: Route + Default + 'static>(&mut self, route: Arc<T>) -> &mut Self {
        let type_id = TypeId::of::<T>();
        self.routes.insert(type_id, route);
        self
    }

    /// ```rust
    /// Adds a default instance of a type that implements the `Route` trait to the underlying data structure
    /// and returns a mutable reference to the current object for method chaining.
    ///
    /// This method requires the generic type `T` to implement the following constraints:
    /// - `Route`: Ensures the type can be treated as a route.
    /// - `Default`: Allows the creation of a default instance for the type.
    /// - `'static`: Ensures the type has a static lifetime, meaning it does not contain any non-'static references.
    ///
    /// The method internally wraps the default instance of `T` in an `Arc` (Atomic Reference Counted Pointer)
    /// before adding it, ensuring thread-safe sharing of the instance.
    ///
    /// # Type Parameters
    /// - `T`: A type that implements `Route`, `Default`, and is `'static`.
    ///
    /// # Returns
    /// - A mutable reference to `self`, allowing method chaining.
    ///
    /// # Example
    /// ```rust
    /// struct MyRoute;
    ///
    /// impl Route for MyRoute {}
    /// impl Default for MyRoute {
    ///     fn default() -> Self {
    ///         MyRoute
    ///     }
    /// }
    ///
    /// let mut obj = YourType::new();
    /// obj.orchestra::<MyRoute>();
    /// ```
    /// ```
    pub fn orchestra<T: Route + Default + 'static>(&mut self) -> &mut Self {
        self.add(Arc::new(T::default()))
    }
    pub fn orchestra_scope<T: Route + Default + 'static, S: RouteScope + Default>(
        &mut self,
    ) -> &mut Self {
        let scope_id = TypeId::of::<S>();
        let route_id = TypeId::of::<T>();
        {
            let entry = self
                .scopes
                .entry(scope_id)
                .or_insert_with(|| (Arc::new(S::default()), Arc::new(Vec::new())));

            let routes_vec_arc = &mut entry.1;
            let mut new_routes = (**routes_vec_arc).clone();
            if !new_routes.contains(&route_id) {
                new_routes.push(route_id);
                *routes_vec_arc = Arc::new(new_routes);
            }
        }
        {
            let scope_ids_arc = self
                .route_scope
                .entry(route_id)
                .or_insert_with(|| Arc::new(Vec::new()));

            let mut new_scopes = (**scope_ids_arc).clone();
            if !new_scopes.contains(&scope_id) {
                new_scopes.push(scope_id);
                *scope_ids_arc = Arc::new(new_scopes);
            }
        }
        self.orchestra::<T>();
        self
    }

    pub fn remove_scope<T: RouteScope + Default>(
        &mut self,
    ) -> Option<(Arc<dyn RouteScope>, Arc<Vec<TypeId>>)> {
        let scope_id = TypeId::of::<T>();
        if let Some(removed_data) = self.scopes.remove(&scope_id) {
            let (_, route_ids_arc) = &removed_data;
            for r_id in route_ids_arc.iter() {
                if let Some(scope_list_arc) = self.route_scope.get_mut(r_id) {
                    let mut new_scope_list: Vec<TypeId> = (**scope_list_arc).clone();
                    new_scope_list.retain(|&id| id != scope_id);
                    if new_scope_list.is_empty() {
                        self.route_scope.remove(r_id);
                    } else {
                        *scope_list_arc = Arc::new(new_scope_list);
                    }
                }
            }
            return Some(removed_data);
        }
        None
    }

    pub fn scope_of<T: RouteScope + Default>(
        &self,
    ) -> Option<(Arc<dyn RouteScope>, Arc<Vec<TypeId>>)> {
        self.scopes
            .get(&TypeId::of::<T>())
            .map(|(d, a)| (d.clone(), a.clone()))
    }

    pub fn route_scope<T: Route + Default + 'static>(&self) -> Option<Arc<dyn RouteScope>> {
        let route_id = TypeId::of::<T>();
        self.route_scope
            .get(&route_id)
            .and_then(|scope_ids| scope_ids.first())
            .and_then(|s_id| self.scopes.get(s_id))
            .map(|(s_obj, _)| s_obj.clone())
    }

    /// Configures the current instance with an ensemble defined by the given `Orchestrator` type.
    ///
    /// This method utilizes the associated `ensemble` function of the specified `Orchestrator` type
    /// to modify and return the mutable reference of the current instance.
    ///
    /// # Type Parameters
    /// - `T`: A type that implements the `Orchestrator` trait. The type must provide its own
    ///        implementation of the `ensemble` method that dictates how the instance should be configured.
    ///
    /// # Returns
    /// A mutable reference to `Self`, allowing for method chaining.
    ///
    /// # Example
    /// ```
    /// struct MyOrchestrator;
    ///
    /// impl Orchestrator for MyOrchestrator {
    ///     fn ensemble(instance: &mut Self) -> &mut Self {
    ///         // Custom configuration logic here
    ///         instance
    ///     }
    /// }
    ///
    /// let mut instance = MyType::new();
    /// instance.ensembles::<MyOrchestrator>();
    /// ```
    ///
    /// # Note
    /// Ensure that the `Orchestrator` implementation provided as the type parameter
    /// contains the necessary logic to modify the instance as desired.
    pub fn ensembles<T: Orchestrator>(&mut self) -> &mut Self {
        T::ensemble(self);
        self
    }

    pub fn default_service<F, U>(&mut self, f: F) -> &mut Self
    where
        F: IntoServiceFactory<U, ServiceRequest> + Clone + Send + Sync + 'static,
        U: ServiceFactory<
                ServiceRequest,
                Config = (),
                Response = ServiceResponse<BoxBody>,
                Error = Error,
            > + 'static,
        U::Service: 'static,
        U::InitError: Debug,
    {
        self.default_service = Some(Arc::new(move |cfg| {
            cfg.default_service(f.clone());
        }));

        self
    }

    pub fn remove_default_service(&mut self) -> &mut Self {
        self.default_service = None;
        self
    }

    /// Configures and mounts all routes associated with the current service.
    ///
    /// This function iterates through all the routes stored in the `self.routes`
    /// collection. For each route, it calls the `mount` method to attach
    /// the route to the provided service configuration.
    ///
    /// # Parameters
    /// - `cfg`: A mutable reference to a [`ServiceConfig`] instance where the routes
    ///   will be mounted. [`ServiceConfig`] is presumed to handle the setup and
    ///   configuration of service routes.
    ///
    /// # Behavior
    /// - Each route in `self.routes` is expected to have a `mount` method that
    ///   takes care of incorporating itself into the provided `ServiceConfig`.
    /// - The function ensures that all the routes from the current service object
    ///   are properly registered in the service configuration.
    ///
    /// # Example Usage
    /// ```rust
    /// let mut service_config = ServiceConfig::new();
    /// let my_service = MyService::new();
    /// my_service.conduct(&mut service_config);
    /// ```
    pub fn conduct(&self, cfg: &mut ServiceConfig) {
        use std::collections::HashMap;
        let mut groups: HashMap<String, (Option<Arc<dyn RouteScope>>, Vec<Arc<dyn Route>>)> =
            HashMap::new();

        for (route_id, route_instance) in self.routes.iter() {
            let scope_ids = self.route_scope.get(route_id);

            if let Some(s_ids) = scope_ids {
                for s_id in s_ids.iter() {
                    if let Some((scope_obj, _)) = self.scopes.get(s_id) {
                        let raw_prefix = scope_obj.prefix().trim();
                        let mut clean_prefix = raw_prefix.trim_end_matches('/').to_string();
                        if !clean_prefix.starts_with('{') && !clean_prefix.starts_with('/') {
                            clean_prefix = format!("/{}", clean_prefix.trim_start_matches('/'));
                        } else if clean_prefix.is_empty() {
                            clean_prefix = "/".to_string();
                        }
                        let prefix = clean_prefix;
                        let entry = groups
                            .entry(prefix)
                            .or_insert((Some(scope_obj.clone()), Vec::new()));
                        entry.1.push(route_instance.clone());
                    }
                }
            } else {
                let entry = groups.entry("/".to_string()).or_insert((None, Vec::new()));
                entry.1.push(route_instance.clone());
            }
        }

        let mut sorted_prefixes: Vec<String> = groups.keys().cloned().collect();
        sorted_prefixes.sort_by(|a, b| b.len().cmp(&a.len()));

        for prefix in sorted_prefixes {
            let (scope_opt, list_rute) = groups.get(&prefix).unwrap();

            match scope_opt {
                None => {
                    for r in list_rute {
                        r.mount(cfg, &prefix);
                    }
                }
                Some(scope_struct) => {
                    let mut scope = scope_struct.wrap(web::scope(&prefix));
                    let routes_for_closure = list_rute.clone();
                    let p_copy = prefix.clone();
                    cfg.service(scope.configure(move |child_cfg| {
                        for r in routes_for_closure {
                            r.mount(child_cfg, &p_copy);
                        }
                    }));
                }
            }
        }
        if let Some(service) = self.default_service.as_ref() {
            (service)(cfg);
        }
    }

    /// Retrieves a reference to all the routes stored within the current instance.
    ///
    /// # Returns
    /// A reference to a `Vec` containing `Arc`s of objects implementing the `Route` trait.
    ///
    /// # Example
    /// ```rust
    /// // Assuming `router` is an instance of a struct that implements this method
    /// let routes = router.all();
    /// for route in routes {
    ///     // Work with each route
    /// }
    /// ```
    ///
    /// # Notes
    /// - The returned reference points to routes that are wrapped in `Arc` for shared ownership.
    /// - Since a shared reference is returned, modifications to the routes are not allowed.
    ///
    /// # Use Case
    /// This method is useful for accessing all the routes in a read-only manner, for operations
    /// like iterating over or inspecting the routes.
    pub fn all(&self) -> &HashMap<TypeId, Arc<dyn Route>> {
        &self.routes
    }
}

impl Default for Routes {
    fn default() -> Self {
        Self::new()
    }
}
