use crate::cores::system::error::{Error, ResultError};
use std::net::IpAddr;

#[derive(Debug)]
pub struct Ip;

#[derive(Debug, Clone, Copy)]
pub enum IpVersion {
    V4,
    V6,
}

/// IP address related utilities.
impl Ip {
    /// Check if an IP address is a bogon (loopback address).
    /// # Arguments
    /// * `ip` - A string slice that holds the IP address.
    /// # Returns
    /// * `bool` - True if the IP address is a bogon, false otherwise.
    /// # Examples
    /// ```rust
    /// use crate::cores::net::ip::Ip;
    /// let ip_v4 = "127.0.0.1";
    /// assert!(Ip::is_bogon(ip_v4));
    /// let ip_v6 = "::1";
    /// assert!(Ip::is_bogon(ip_v6));
    /// ```
    pub fn is_bogon<Ip: AsRef<str>>(ip: Ip) -> bool {
        let Ok(ip) = ip.as_ref().parse::<IpAddr>() else {
            return false;
        };

        match ip {
            IpAddr::V4(v4) => {
                v4.is_private()
                    || v4.is_loopback()
                    || v4.is_link_local()
                    || v4.is_multicast()
                    || v4.is_documentation()
                    || v4.octets()[0] == 0 // Broadcast/Current network
            }
            IpAddr::V6(v6) => {
                v6.is_loopback()
                    || v6.is_unspecified()
                    || v6.is_multicast()
                    || (v6.segments()[0] & 0xfe00) == 0xfc00
            }
        }
    }

    /// Determine the version of an IP address (IPv4 or IPv6).
    /// # Arguments
    /// * `ip` - A string slice that holds the IP address.
    /// # Returns
    /// * `Result<IpVersion, Error>` - The version of the IP address or an error.
    /// # Examples
    /// ```rust
    /// use crate::cores::net::ip::{Ip, IpVersion};
    /// let ip_v4 = "127.0.0.1";
    /// let version_v4 = Ip::version(ip_v4).unwrap();
    /// assert_eq!(version_v4, IpVersion::V4);
    /// let ip_v6 = "::1";
    /// let version_v6 = Ip::version(ip_v6).unwrap();
    /// assert_eq!(version_v6, IpVersion::V6);
    /// ```
    pub fn version<Ip: AsRef<str>>(ip: Ip) -> ResultError<IpVersion> {
        let Ok(ip_a) = ip.as_ref().parse::<IpAddr>() else {
            return Err(Error::invalid_input(format!(
                "Invalid IP address: {}",
                ip.as_ref()
            )));
        };
        match ip_a {
            IpAddr::V4(_) => Ok(IpVersion::V4),
            IpAddr::V6(_) => Ok(IpVersion::V6),
        }
    }

    /// Determine if Ip is public
    /// # Arguments
    /// * `ip` - A string slice that holds the IP address.
    /// # Returns
    /// * `Option<IpVersion>` - indicates the result is valid public IP Address, returning IP address
    pub fn public_ip_version<Ip: AsRef<str>>(ip: Ip) -> Option<IpVersion> {
        if let Ok(e) = Self::version(&ip) && !Self::is_bogon(&ip) {
            return Some(e)
        }
        None
    }

    /// Convert an IP address to its numeric representation.
    /// /// # Arguments
    /// * `ip` - A string slice that holds the IP address.
    /// /// # Returns
    /// * `Result<u128, Error>` - The numeric representation of the IP address or an error.
    /// /// # Examples
    /// ```rust
    /// use crate::cores::net::ip::{Ip, IpVersion};
    /// let ip_v4 = "127.0.0.1";
    /// let long_v4 = Ip::ip_to_long(ip_v4).unwrap();
    /// assert_eq!(long_v4, 2130706433);
    /// let ip_v6 = "::1";
    /// let long_v6 = Ip::ip_to_long(ip_v6).unwrap();
    /// assert_eq!(long_v6, 1);
    /// ```
    pub fn ip_to_long<Ip: AsRef<str>>(ip: Ip) -> ResultError<u128> {
        let ip_str = ip.as_ref();
        let addr: IpAddr = ip_str
            .parse()
            .map_err(|e| Error::parse_error(format!("Failed to parse IP address: {}", ip_str)))?;

        match addr {
            IpAddr::V4(v4) => Ok(u32::from(v4) as u128),
            IpAddr::V6(v6) => Ok(u128::from(v6)),
        }
    }

    /// Normalize an IPv4 address by removing leading zeros and converting it to a string.
    /// # Arguments
    /// * `ip` - A string slice that holds the IP address.
    /// # Returns
    /// * `Result<String, Error>` - The normalized IP address or an error.
    /// # Examples
    /// ```rust
    /// use crate::cores::net::ip::Ip;
    /// let ip_v4 = "000.000.000.001";
    /// let normalized_v4 = Ip::normalize_ip4(ip_v4).unwrap();
    /// assert_eq!(normalized_v4, "0.0.0.0");
    /// ```
    pub fn normalize_ip4<T: AsRef<str>>(ip: T) -> ResultError<String> {
        let ip = ip.as_ref();
        let mut ips = Vec::new();
        for p in ip.split('.') {
            if ip.len() > 3 || ips.len() == 4 {
                return Err(Error::invalid_input("Invalid IP address"));
            }
            let num = p
                .parse::<isize>()
                .map_err(|_| Error::invalid_input("Invalid IP address"))?;
            if num < 0 || num > 255 {
                return Err(Error::invalid_input("Invalid IP address"));
            }
            ips.push(num.to_string());
        }
        Ok(ips.join("."))
    }

    /// Convert a numeric representation of an IP address back to its string form.
    /// # Arguments
    /// * `long` - The numeric representation of the IP address.
    /// * `version` - The version of the IP address (IPv4 or IPv6).
    /// # Returns
    /// * `Result<String, Error>` - The string representation of the IP address or an error.
    /// # Examples
    /// ```rust
    /// use crate::cores::net::ip::{Ip, IpVersion};
    /// let long_v4 = 2130706433;
    /// let ip_v4 = Ip::long_to_ip(long_v4, IpVersion::V4).unwrap();
    /// assert_eq!(ip_v4, "127.0.0.1");
    /// let long_v6 = 1;
    /// let ip_v6 = Ip::long_to_ip(long_v6, IpVersion::V6).unwrap();
    /// assert_eq!(ip_v6, "::1");
    /// ```
    pub fn long_to_ip(long: u128, version: IpVersion) -> ResultError<String> {
        match version {
            IpVersion::V4 => {
                if long > u32::MAX as u128 {
                    return Err(Error::invalid_input("Overflow IPv4"));
                }
                Ok(std::net::Ipv4Addr::from(long as u32).to_string())
            }
            IpVersion::V6 => Ok(std::net::Ipv6Addr::from(long).to_string()),
        }
    }

    fn get_mask_v4(prefix: u32) -> u32 {
        if prefix == 0 {
            0
        } else {
            u32::MAX << (32 - prefix)
        }
    }

    fn get_mask_v6(prefix: u32) -> u128 {
        if prefix == 0 {
            0
        } else {
            u128::MAX << (128 - prefix)
        }
    }
    pub fn range_to_bounds_v4<T: AsRef<str>, E: AsRef<str>>(
        start_ip: T,
        end_ip: E,
    ) -> ResultError<(String, String)> {
        let start = u32::from(
            start_ip
                .as_ref()
                .parse::<std::net::Ipv4Addr>()
                .map_err(|_| Error::invalid_input("Invalid Start IP"))?,
        );
        let end = u32::from(
            end_ip
                .as_ref()
                .parse::<std::net::Ipv4Addr>()
                .map_err(|_| Error::invalid_input("Invalid End IP"))?,
        );

        if start > end {
            return Err(Error::invalid_range("Start IP must be less than End IP"));
        }

        Ok((
            std::net::Ipv4Addr::from(start).to_string(),
            std::net::Ipv4Addr::from(end).to_string(),
        ))
    }

    pub fn ipv4_cidr_to_range<T: AsRef<str>>(cidr: T) -> ResultError<(String, String)> {
        let cidr_str = cidr.as_ref();
        let parts: Vec<&str> = cidr_str.split('/').collect();
        if parts.len() != 2 {
            return Err(Error::invalid_input("Format must be IP/Prefix"));
        }

        let ip_u32 = u32::from(
            parts[0]
                .parse::<std::net::Ipv4Addr>()
                .map_err(|_| Error::invalid_input("Invalid IP"))?,
        );
        let prefix = parts[1]
            .parse::<u32>()
            .map_err(|_| Error::invalid_range("Invalid Prefix"))?;

        if prefix > 32 {
            return Err(Error::invalid_range("Prefix > 32"));
        }

        let mask = Self::get_mask_v4(prefix);
        let first = ip_u32 & mask;
        let last = first | !mask;

        Ok((
            std::net::Ipv4Addr::from(first).to_string(),
            std::net::Ipv4Addr::from(last).to_string(),
        ))
    }
    pub fn ipv6_cidr_to_range<T: AsRef<str>>(cidr: T) -> ResultError<(String, String)> {
        let cidr_str = cidr.as_ref();
        let parts: Vec<&str> = cidr_str.split('/').collect();
        if parts.len() != 2 {
            return Err(Error::invalid_input("Format must be IP/Prefix"));
        }

        let ip_u128 = u128::from(
            parts[0]
                .parse::<std::net::Ipv6Addr>()
                .map_err(|_| Error::invalid_input("Invalid IPv6"))?,
        );
        let prefix = parts[1]
            .parse::<u32>()
            .map_err(|_| Error::invalid_range("Invalid Prefix"))?;

        if prefix > 128 {
            return Err(Error::invalid_range("Prefix > 128"));
        }

        let mask = Self::get_mask_v6(prefix);
        let first = ip_u128 & mask;
        let last = first | !mask;

        Ok((
            std::net::Ipv6Addr::from(first).to_string(),
            std::net::Ipv6Addr::from(last).to_string(),
        ))
    }

    pub fn ipv4_to_cidr_string<T: AsRef<str>, E: AsRef<str>>(
        start_ip: T,
        end_ip: E,
    ) -> ResultError<String> {
        let start = u32::from(
            start_ip
                .as_ref()
                .parse::<std::net::Ipv4Addr>()
                .map_err(|_| Error::invalid_input("Invalid Start IP"))?,
        );
        let end = u32::from(
            end_ip
                .as_ref()
                .parse::<std::net::Ipv4Addr>()
                .map_err(|_| Error::invalid_input("Invalid End IP"))?,
        );

        if start > end {
            return Err(Error::invalid_range("Start IP must be less than End IP"));
        }

        let diff = start ^ end;
        let prefix = if diff == 0 {
            32
        } else {
            // leading_zeros()
            diff.leading_zeros()
        };

        let base_ip = std::net::Ipv4Addr::from(start & Self::get_mask_v4(prefix));
        Ok(format!("{}/{}", base_ip, prefix))
    }

    pub fn ipv6_to_cidr_string<T: AsRef<str>, E: AsRef<str>>(
        start_ip: T,
        end_ip: E,
    ) -> ResultError<String> {
        let start = u128::from(
            start_ip
                .as_ref()
                .parse::<std::net::Ipv6Addr>()
                .map_err(|_| Error::invalid_input("Invalid Start IPv6"))?,
        );
        let end = u128::from(
            end_ip
                .as_ref()
                .parse::<std::net::Ipv6Addr>()
                .map_err(|_| Error::invalid_input("Invalid End IPv6"))?,
        );

        if start > end {
            return Err(Error::invalid_range("Start IP must be less than End IP"));
        }

        let diff = start ^ end;
        let prefix = if diff == 0 { 128 } else { diff.leading_zeros() };

        let base_ip = std::net::Ipv6Addr::from(start & Self::get_mask_v6(prefix));
        Ok(format!("{}/{}", base_ip, prefix))
    }
}
