//! Management of (command line) options for client and server.

use std::fmt::{Display, Formatter};
use std::net::SocketAddr;
use std::str::FromStr;

/// Basic options for client and server.
#[derive(Debug)]
pub struct Options {
    /// The source port
    pub port: u16,
    /// Transition probabilities for packet loss "simulation" via a markov chain (not lost->lost, not lost->not lost)
    pub transition_probabilities: (f64, f64),
}

impl Options {
    /// Get options from given t, p and q. Use defaults if none is given.
    pub fn parse(t: Option<&str>, p: Option<&str>, q: Option<&str>) -> Result<Self, &'static str> {
        Ok(Options {
            port: parse_t(t)?,
            transition_probabilities: parse_p_q(p, q)?,
        })
    }
}

impl Display for Options {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Options: 'source port {} with transition probabilities p={} and q={} for markov chain'", self.port, self.transition_probabilities.0, self.transition_probabilities.1)
    }
}

/// Get port number from given t. Uses default if none is given.
fn parse_t(t: Option<&str>) -> Result<u16, &'static str> {
    const DEFAULT_PORT: u16 = 42424;

    if let Some(s) = t {
        return match s.parse::<u16>() {
            Err(_) => Err("Couldn't parse port number."), //TODO don't ignore error kind
            Ok(port) if port < 1024 => Err("Invalid port number."),
            Ok(port) => Ok(port),
        };
    } else {
        return Ok(DEFAULT_PORT);
    }
}

/// Get transition probabilities for the markov chain from given p and q. Uses default if none is given.
fn parse_p_q(p: Option<&str>, q: Option<&str>) -> Result<(f64, f64), &'static str> {
    const DEFAULT_P: f64 = 0.0;
    const DEFAULT_Q: f64 = 0.0;

    let mut r = (DEFAULT_P, DEFAULT_Q);

    if let Some(s) = p {
        match s.parse::<f64>() {
            Err(_) => return Err("Couldn't parse p."),
            Ok(x) if x > 1.0 || x < 0.0 => return Err("p must lie between 0 and 1."),
            Ok(x) if q.is_none() => return Ok((x, 1.0 - x)),
            Ok(x) => r.0 = x,
        }
    }

    if let Some(s) = q {
        match s.parse::<f64>() {
            Err(_) => return Err("Couldn't parse q."),
            Ok(x) if x > 1.0 || x < 0.0 => return Err("q must lie between 0 and 1."),
            Ok(x) if p.is_none() => return Ok((1.0 - x, x)),
            Ok(x) => r.1 = x,
        }
    }

    return Ok(r);
}

/// Get socket address from given host. Return error if none is given.
pub fn parse_host(host: &str) -> Result<SocketAddr, &'static str> {
    return match SocketAddr::from_str(host) {
        Err(_) => Err("Couldn't parse host."),
        Ok(socket_addr) => Ok(socket_addr),
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
