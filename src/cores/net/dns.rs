use crate::cores::system::error::{Error, ResultError};
use hickory_resolver::config::{LookupIpStrategy, ResolverConfig, ResolverOpts};
use hickory_resolver::lookup::{Ipv4Lookup, Ipv6Lookup, Lookup, MxLookup, NsLookup, TxtLookup};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::proto::rr::RecordType;
use hickory_resolver::proto::runtime::TokioRuntimeProvider;
use hickory_resolver::{IntoName, ResolverBuilder, TokioResolver};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dns {
    GoogleDoh,
    CloudflareDoh,
    Quad9Doh,
    GoogleDns,
    CloudflareDns,
    Quad9Dns,
}

static RESOLVER_CACHE: LazyLock<RwLock<HashMap<Dns, Arc<TokioResolver>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

impl Dns {
    /// Returns the `ResolverConfig` associated with the `Dns` variant.
    ///
    /// This function matches the current `Dns` enum variant and returns
    /// the corresponding `ResolverConfig`. It supports configurations for
    /// various DNS providers, including both DoH (DNS over HTTPS) and traditional
    /// DNS modes.
    ///
    /// # Variants and Respective Configurations
    /// - `Dns::CloudflareDoh`: Returns the DoH configuration for Cloudflare.
    /// - `Dns::GoogleDoh`: Returns the DoH configuration for Google.
    /// - `Dns::Quad9Doh`: Returns the DoH configuration for Quad9.
    /// - `Dns::GoogleDns`: Returns the traditional DNS configuration for Google.
    /// - `Dns::CloudflareDns`: Returns the traditional DNS configuration for Cloudflare.
    /// - `Dns::Quad9Dns`: Returns the traditional DNS configuration for Quad9.
    ///
    /// # Returns
    /// A `ResolverConfig` instance corresponding to the `Dns` variant.
    ///
    /// # Examples
    /// ```rust
    /// let dns = Dns::CloudflareDoh;
    /// let config = dns.config();
    /// assert_eq!(config, ResolverConfig::cloudflare_https());
    /// ```
    pub fn config(&self) -> ResolverConfig {
        match self {
            Dns::CloudflareDoh => ResolverConfig::cloudflare_https(),
            Dns::GoogleDoh => ResolverConfig::google_https(),
            Dns::Quad9Doh => ResolverConfig::quad9_https(),
            Dns::GoogleDns => ResolverConfig::google(),
            Dns::CloudflareDns => ResolverConfig::cloudflare(),
            Dns::Quad9Dns => ResolverConfig::quad9(),
        }
    }

    /// Creates a `ResolverBuilder` configured to use a `TokioConnectionProvider`
    /// with custom resolver options.
    ///
    /// This function initializes a `ResolverBuilder` that is configured to:
    /// - Use IPv4 addresses exclusively when performing DNS lookups.
    /// - Utilize a Tokio-based runtime for handling asynchronous network connections.
    ///
    /// # Returns
    ///
    /// A `ResolverBuilder` instance configured with:
    /// - A connection provider based on the `TokioRuntimeProvider`.
    /// - Resolver options (`ResolverOpts`) set to use the `Ipv4Only` IP lookup strategy.
    ///
    /// # Example
    ///
    /// ```rust
    /// let resolver_builder = my_instance.builder();
    /// ```
    ///
    /// # Dependencies
    ///
    /// - The function relies on the `TokioConnectionProvider` and `TokioRuntimeProvider`
    ///   to provide asynchronous connection capabilities.
    /// - The DNS resolver is customized using the `ResolverOpts` structure.
    ///
    /// # Notes
    ///
    /// Ensure the Tokio runtime is properly set up in your application, as this implementation
    /// depends on Tokio's asynchronous features to function correctly.
    pub fn builder(&self) -> ResolverBuilder<TokioConnectionProvider> {
        let mut opt = ResolverOpts::default();
        opt.ip_strategy = LookupIpStrategy::Ipv4Only;
        // dns lookup timeout is 5 seconds
        opt.timeout = Duration::from_secs(5);
        let provider = TokioConnectionProvider::new(TokioRuntimeProvider::new());
        TokioResolver::builder_with_config(self.config(), provider).with_options(opt)
    }

    /// Resolves and returns an `Arc<TokioResolver>` instance for the current object.
    ///
    /// This function checks the shared `RESOLVER_CACHE` to determine if the resolver
    /// for the current object already exists. If a cached resolver is found, it is
    /// returned directly. Otherwise, a new resolver is created using the `builder`
    /// method, inserted into the cache, and subsequently returned.
    ///
    /// # Returns
    /// An `Arc<TokioResolver>` instance associated with the current object.
    ///
    /// # Panics
    /// This method will panic if:
    /// - The `RESOLVER_CACHE` lock is poisoned due to another thread panicking while
    ///   holding the lock.
    /// - Any other unexpected behavior that could compromise proper write or read
    ///   operations on the cache.
    ///
    /// # Thread Safety
    /// This method is thread-safe due to the use of `RwLock` for synchronizing access
    /// to `RESOLVER_CACHE`.
    ///
    /// # Example
    /// ```rust
    /// let resolver = my_instance.resolver();
    /// // Use the resolver for DNS resolution or other tasks.
    /// ```
    pub fn resolver(&self) -> Arc<TokioResolver> {
        {
            let cache = RESOLVER_CACHE.read();
            if let Some(resolver) = cache.get(self) {
                return Arc::clone(resolver);
            }
        }

        let mut cache = RESOLVER_CACHE.write();
        if let Some(resolver) = cache.get(self) {
            return Arc::clone(resolver);
        }

        let resolver = Arc::new(self.builder().build());
        cache.insert(*self, Arc::clone(&resolver));
        resolver
    }

    /// Resolves a DNS resolver instance from the provided optional resolver or defaults to Google's DoH (DNS over HTTPS) resolver.
    ///
    /// # Arguments
    ///
    /// * `dns` - An `Option` containing an `Arc` of `TokioResolver`. If `Some`, the provided resolver is cloned and returned.
    /// If `None`, it defaults to using `Self::GoogleDoh.resolver()`.
    ///
    /// # Returns
    ///
    /// * An `Arc<TokioResolver>` instance. The returned instance is either the provided `dns` resolver (if `Some`) or
    /// the default Google DoH resolver (if `None`).
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use some_module::TokioResolver; // Replace `some_module` with the actual module path
    ///
    /// let custom_resolver: Option<Arc<TokioResolver>> = Some(Arc::new(TokioResolver::new()));
    /// let resolver = dns_resolver_or(custom_resolver);
    /// // `resolver` now holds the provided custom resolver.
    ///
    /// let default_resolver = dns_resolver_or(None);
    /// // `default_resolver` now holds the default Google DoH resolver.
    /// ```
    pub fn resolver_of(dns: Option<Arc<TokioResolver>>) -> Arc<TokioResolver> {
        match dns {
            Some(dns) => dns.clone(),
            None => Self::GoogleDoh.resolver(),
        }
    }

    /// Retrieves DNS records for a specified domain and record type.
    ///
    /// This asynchronous function performs a DNS lookup to retrieve records of a specified type
    /// (e.g., A, AAAA, CNAME, etc.) for a given domain. The function utilizes a `TokioResolver`
    /// for the DNS resolution. If no resolver is explicitly provided, it defaults to using the
    /// resolver associated with `self`.
    ///
    /// # Parameters
    ///
    /// * `domain` - The domain name for which DNS records are to be retrieved. The type of `domain`
    ///   must implement the `IntoName` trait, which allows various representations of domain names
    ///   to be passed in.
    /// * `record` - The type of DNS record to look up. This is specified using the `RecordType` enum.
    /// * `dns` - An optional reference to an `Arc<TokioResolver>`. If provided, this resolver will
    ///   be used for the DNS lookup. If `None` is passed, the method defaults to using the resolver
    ///   associated with `self`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// * `Lookup` - On success, a lookup object containing the requested DNS records.
    /// * `ResolveError` - On failure, an error describing why the DNS resolution failed.
    ///
    /// # Errors
    ///
    /// Returns a `ResolveError` if:
    /// 1. The DNS resolution process encounters an issue.
    /// 2. The requested `domain` or `record` type is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use trust_dns_resolver::proto::rr::RecordType;
    /// use trust_dns_resolver::TokioAsyncResolver;
    /// use std::sync::Arc;
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let resolver = Arc::new(TokioAsyncResolver::from_system_conf()?);
    ///     let client = YourDnsClient::new(resolver.clone());
    ///
    ///     let domain = "example.com";
    ///     let record_type = RecordType::A;
    ///
    ///     let result = client.get_dns_records(domain, record_type, Some(&resolver)).await?;
    ///     println!("DNS records: {:?}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// * This function is asynchronous and requires an async runtime such as Tokio.
    /// * Ensure that the `TokioResolver` provided, if any, is in a valid and initialized state.
    pub async fn dns_records<N>(&self, name: N, record: RecordType) -> ResultError<Lookup>
    where
        N: IntoName,
    {
        self.resolver().lookup(name, record).await.map_err(|e|Error::from_error(e))
    }

    /// Performs an MX (Mail Exchange) record lookup for a given domain name.
    ///
    /// This asynchronous function retrieves the MX records associated with the specified
    /// domain name using the provided optional DNS resolver. If no custom resolver is
    /// supplied, it falls back to a default internal resolver.
    ///
    /// # Type Parameters
    /// - `N`: A type that implements the `IntoName` trait, representing the domain name to query.
    ///
    /// # Parameters
    /// - `name`: The domain name for which to perform the MX record lookup. It must implement the `IntoName` trait.
    /// - `dns`: An optional `Arc`-wrapped instance of a `TokioResolver` to use as the DNS resolver.
    ///   If `None`, the default resolver is used.
    ///
    /// # Returns
    /// - `Ok(MxLookup)`: On success, returns an `MxLookup` object containing the retrieved MX records.
    /// - `Err(ResolveError)`: On failure, returns a `ResolveError` indicating the cause of the failure
    ///   (e.g., network issues, invalid domain name, or no MX records found).
    ///
    /// # Errors
    /// This function will return a `ResolveError` if:
    /// - The domain name cannot be resolved.
    /// - There is an issue with the DNS query, such as network errors or invalid DNS responses.
    ///
    /// # Examples
    /// ```rust
    /// use std::sync::Arc;
    /// use trust_dns_resolver::TokioAsyncResolver;
    ///
    /// async fn lookup_mx_records() -> Result<(), Box<dyn std::error::Error>> {
    ///     let domain_name = "example.com";
    ///     let resolver = Arc::new(TokioAsyncResolver::tokio_from_system_conf()?);
    ///
    ///     let mx_records = mx_records(domain_name, Some(resolver)).await?;
    ///
    ///     println!("Found MX records: {:?}", mx_records);
    ///     Ok(())
    /// }
    /// ```
    pub async fn mx_records<N>(&self, name: N) -> ResultError<MxLookup>
    where
        N: IntoName,
    {
        Ok(self.resolver().mx_lookup(name).await.map_err(Error::from_error)?)
    }

    /// Performs an asynchronous DNS A record lookup for the given domain name.
    ///
    /// # Parameters
    /// - `name`: A value that can be converted into a domain name. This specifies the target domain name to query for A records.
    ///            The type of `name` must implement the `IntoName` trait.
    /// - `dns`: An optional `Arc`-wrapped instance of a `TokioResolver` that can be used for the DNS resolution. If `None` is provided,
    ///           a default resolver will be used.
    ///
    /// # Returns
    /// - `Ok(Ipv4Lookup)`: On successful lookup, returns an `Ipv4Lookup` containing the resolved IPv4 addresses.
    /// - `Err(ResolveError)`: Returns an error if the lookup fails.
    ///
    /// # Type Parameters
    /// - `N`: A generic type that must implement the `IntoName` trait, allowing `N` to be converted into a valid domain name.
    ///
    /// # Example
    /// ```rust
    /// use std::sync::Arc;
    /// use trust_dns_resolver::TokioAsyncResolver;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let resolver = Arc::new(TokioAsyncResolver::tokio_from_system_conf()?);
    ///     let lookup = a_records("example.com", Some(resolver)).await?;
    ///
    ///     for ip in lookup.iter() {
    ///         println!("Found IP: {}", ip);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Notes
    /// - The function uses a default resolver if no custom `TokioResolver` instance is provided in `dns`.
    /// - The lookup process is asynchronous and requires `await` to complete.
    ///
    /// # Errors
    /// - Returns a `ResolveError` if the domain name cannot be resolved or if there are issues communicating with the DNS server.
    pub async fn a_records<N>(&self, name: N) -> ResultError<Ipv4Lookup>
    where
        N: IntoName,
    {
        Ok(self.resolver().ipv4_lookup(name).await.map_err(Error::from_error)?)
    }

    /// Performs a DNS AAAA (IPv6) record lookup for the given domain name.
    ///
    /// # Parameters
    /// - `name`: The domain name to resolve, implementing the `IntoName` trait.
    ///           This is typically a string or type that can be converted into a
    ///           valid domain name.
    /// - `dns`: An optional `Arc`-wrapped instance of `TokioResolver` to use for
    ///          the lookup. If `None`, a default resolver will be used.
    ///
    /// # Returns
    /// - `Result<Ipv6Lookup, ResolveError>`:
    ///   - On success, returns an `Ipv6Lookup` containing the resolved IPv6 addresses.
    ///   - On failure, returns a `ResolveError` describing the error encountered
    ///     during the lookup.
    ///
    /// # Errors
    /// This function will return an error in the following circumstances:
    /// - If the domain name cannot be resolved into a valid IPv6 address.
    /// - If there are network issues or other internal errors during the lookup process.
    ///
    /// # Example
    /// ```rust
    /// use std::sync::Arc;
    /// use trust_dns_resolver::TokioAsyncResolver;
    ///
    /// # async fn example() -> Result<(), ResolveError> {
    /// let resolver = TokioAsyncResolver::tokio_from_system_conf().unwrap();
    /// let ipv6_lookup = MyResolver::aaa_records("example.com", Some(Arc::new(resolver))).await?;
    ///
    /// for ip in ipv6_lookup.iter() {
    ///     println!("Resolved IPv6 address: {}", ip);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Note
    /// This function is asynchronous and requires the caller to be running
    /// within a Tokio runtime. To execute this, ensure you have `async` context
    /// available.
    pub async fn aaa_records<N>(&self, name: N) -> ResultError<Ipv6Lookup>
    where
        N: IntoName,
    {
        Ok(self.resolver().ipv6_lookup(name).await.map_err(Error::from_error)?)
    }

    /// Performs a TXT record DNS query for the given domain name.
    ///
    /// # Parameters
    /// - `name`: The domain name for which the TXT record lookup should be performed.
    ///   This should implement the `IntoName` trait.
    /// - `dns`: An optional `Arc<TokioResolver>` instance to be used for the DNS resolution.
    ///   If `None` is provided, the function will use the default DNS resolver.
    ///
    /// # Returns
    /// - `Ok(TxtLookup)`: A successful result containing the `TxtLookup` object,
    ///   which includes the TXT records associated with the provided domain name.
    /// - `Err(ResolveError)`: An error occurred during the DNS resolution process.
    ///
    /// # Examples
    /// ```rust
    /// use std::sync::Arc;
    /// use trust_dns_resolver::TokioAsyncResolver;
    ///
    /// async fn perform_txt_lookup() {
    ///     let resolver = TokioAsyncResolver::tokio_from_system_conf().unwrap();
    ///     match txt_records("example.com", Some(Arc::new(resolver))).await {
    ///         Ok(lookup) => {
    ///             for txt in lookup.iter() {
    ///                 println!("TXT record: {:?}", txt);
    ///             }
    ///         },
    ///         Err(err) => eprintln!("Failed to resolve TXT records: {:?}", err),
    ///     }
    /// }
    /// ```
    ///
    /// # Notes
    /// - The function is asynchronous and requires the `.await` syntax when called.
    /// - It uses the resolver provided via the `dns` parameter, or initializes a default
    ///   resolver if none is passed.
    /// - This function internally calls the `txt_lookup` method on the resolver and propagates
    ///   any errors encountered during the lookup.
    pub async fn txt_records<N>(&self, name: N) -> ResultError<TxtLookup>
    where
        N: IntoName,
    {
        Ok(self.resolver().txt_lookup(name).await.map_err(Error::from_error)?)
    }

    /// Performs a nameserver (NS) record lookup for the given domain name.
    ///
    /// # Parameters
    /// - `name`: The domain name for which the NS records are to be looked up.
    ///           Must implement the `IntoName` trait, which allows conversion into a domain name type.
    /// - `dns`: An optional `Arc<TokioResolver>` instance. If provided, this custom DNS resolver
    ///          will be used for the lookup. If `None`, the default resolver will be used.
    ///
    /// # Returns
    /// - `Result<NsLookup, ResolveError>`: On success, returns an `NsLookup` structure containing
    ///   the resolved NS records. On failure, returns a `ResolveError` with details about the
    ///   failure.
    ///
    /// # Errors
    /// This function returns a `ResolveError` if:
    /// - The provided domain name is invalid.
    /// - The DNS resolution fails for any reason (e.g., network issues, timeout, or no records found).
    ///
    /// # Examples
    /// ```rust
    /// use std::sync::Arc;
    /// use your_crate::ns_records;
    /// use trust_dns_resolver::TokioAsyncResolver;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let resolver = Arc::new(TokioAsyncResolver::tokio_from_system_conf()?);
    ///     let name = "example.com";
    ///
    ///     let ns_records = ns_records(name, Some(resolver)).await?;
    ///     println!("NS Records: {:?}", ns_records);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn ns_records<N>(&self, name: N) -> ResultError<NsLookup>
    where
        N: IntoName,
    {
        Ok(self.resolver().ns_lookup(name).await.map_err(Error::from_error)?)
    }

    /// Asynchronously resolves a CNAME record for the given domain name.
    ///
    /// This function attempts to perform a DNS lookup for a CNAME record associated with the
    /// provided domain name. If a CNAME record is found, it returns the first record as a `String`.
    /// If no such record exists, it returns `Ok(None)`. If a DNS resolution error occurs, it returns
    /// a `ResolveError`.
    ///
    /// # Type Parameters
    /// - `N`: A type that can be converted into a domain name, implementing the `IntoName` trait.
    ///
    /// # Arguments
    /// - `name`: The domain name for which the CNAME record lookup is performed. It should implement
    ///   the `IntoName` trait.
    /// - `dns`: An optional reference-counted pointer (`Arc`) to a custom `TokioResolver` to be used
    ///   for DNS resolution. If this argument is `None`, a default resolver will be used.
    ///
    /// # Returns
    /// - `Ok(Some(String))`: The first CNAME record as a `String` if one exists.
    /// - `Ok(None)`: If no CNAME record is found for the given domain name.
    /// - `Err(ResolveError)`: If an error occurs during the DNS resolution process.
    ///
    /// # Errors
    /// This function will return an `Err` if there is a failure in looking up the DNS record.
    /// Possible errors include timeouts, network issues, or invalid domain names.
    ///
    /// # Example
    /// ```rust
    /// use std::sync::Arc;
    /// use some_dns_library::{cname_record, TokioResolver};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let resolver = TokioResolver::new().expect("Failed to create resolver");
    ///     let cname = cname_record("example.com", Some(Arc::new(resolver))).await;
    ///
    ///     match cname {
    ///         Ok(Some(record)) => println!("CNAME record: {}", record),
    ///         Ok(None) => println!("No CNAME record found."),
    ///         Err(e) => eprintln!("Error resolving CNAME: {:?}", e),
    ///     }
    /// }
    /// ```
    pub async fn cname_record<N>(&self, name: N) -> ResultError<Option<String>>
    where
        N: IntoName,
    {
        // return single name if exists

        let lookup = self.resolver().lookup(name, RecordType::CNAME).await.map_err(Error::from_error)?;
        Ok(lookup
            .iter()
            .find(|r| r.record_type() == RecordType::CNAME)
            .map(|r| r.to_string()))
    }
}
