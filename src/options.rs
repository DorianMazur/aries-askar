use std::borrow::Cow;
use std::collections::HashMap;

use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};

use super::error::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options<'a> {
    pub schema: Cow<'a, str>,
    pub user: Cow<'a, str>,
    pub password: Cow<'a, str>,
    pub host: Cow<'a, str>,
    pub query: HashMap<String, String>,
    pub fragment: Cow<'a, str>,
}

impl<'a> Options<'a> {
    pub fn parse_uri(uri: &str) -> Result<Options> {
        let mut fragment_and_remain = uri.splitn(2, '#');
        let uri = fragment_and_remain.next().unwrap_or_default();
        let fragment = percent_decode(fragment_and_remain.next().unwrap_or_default());
        let mut schema_and_remain = uri.splitn(2, ':');
        let schema = schema_and_remain.next().unwrap_or_default();

        let (schema, host_and_query) = if let Some(remain) = schema_and_remain.next() {
            if schema.is_empty() {
                ("", uri)
            } else {
                (schema, remain.trim_start_matches("//"))
            }
        } else {
            ("", uri)
        };
        let schema = percent_decode(schema);

        let mut host_and_query = host_and_query.splitn(2, '?');
        let (user, password, host) = {
            let mut user_and_host = host_and_query.next().unwrap_or_default().splitn(2, '@');
            let user_pass = user_and_host.next().unwrap_or_default();
            if let Some(host) = user_and_host.next() {
                let mut user_pass = user_pass.splitn(2, ':');
                let user = percent_decode(user_pass.next().unwrap_or_default());
                let pass = percent_decode(user_pass.next().unwrap_or_default());
                (user, pass, host)
            } else {
                (Cow::Borrowed(""), Cow::Borrowed(""), user_pass)
            }
        };
        let host = percent_decode(host);

        let query = if let Some(query) = host_and_query.next() {
            url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .fold(HashMap::new(), |mut map, (k, v)| {
                    map.insert(k, v);
                    map
                })
        } else {
            HashMap::new()
        };

        Ok(Options {
            user,
            password,
            host,
            schema,
            query,
            fragment,
        })
    }

    pub fn into_uri(self) -> String {
        let mut uri = String::new();
        if !self.schema.is_empty() {
            percent_encode_into(&mut uri, &self.schema);
            uri.push_str("://");
        }
        if !self.user.is_empty() || !self.password.is_empty() {
            percent_encode_into(&mut uri, &self.user);
            uri.push(':');
            percent_encode_into(&mut uri, &self.password);
            uri.push('@');
        }
        percent_encode_into(&mut uri, &self.host);
        if !self.query.is_empty() {
            uri.push('?');
            for (k, v) in self.query {
                push_iter_str(&mut uri, url::form_urlencoded::byte_serialize(k.as_bytes()));
                uri.push('=');
                push_iter_str(&mut uri, url::form_urlencoded::byte_serialize(v.as_bytes()));
            }
        }
        if !self.fragment.is_empty() {
            uri.push('#');
            percent_encode_into(&mut uri, &self.fragment);
        }
        uri
    }
}

#[inline]
fn push_iter_str<'a, I: Iterator<Item = &'a str>>(s: &mut String, iter: I) {
    for item in iter {
        s.push_str(item);
    }
}

#[inline]
fn percent_decode(s: &str) -> Cow<'_, str> {
    percent_decode_str(s).decode_utf8_lossy()
}

// #[inline]
// fn percent_encode(s: &str) -> Cow<'_, str> {
//     let mut result = None;
//     for val in utf8_percent_encode(s, NON_ALPHANUMERIC) {
//         if result.is_none() {
//             result = Some(Cow::Borrowed(val));
//         } else {
//             result.as_mut().unwrap().to_mut().push_str(val);
//         }
//     }
//     result.unwrap_or_default()
// }

#[inline]
fn percent_encode_into(result: &mut String, s: &str) {
    push_iter_str(result, utf8_percent_encode(s, NON_ALPHANUMERIC))
}

pub trait IntoOptions<'a> {
    fn into_options(self) -> Result<Options<'a>>;
}

impl<'a> IntoOptions<'a> for Options<'a> {
    fn into_options(self) -> Result<Options<'a>> {
        Ok(self)
    }
}

impl<'a> IntoOptions<'a> for &'a str {
    fn into_options(self) -> Result<Options<'a>> {
        Options::parse_uri(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::FromIterator;

    #[test]
    fn options_basic() {
        let opts = Options::parse_uri("schema://user%2E:pass@dbname?a+1=b#frag").unwrap();
        let bs = Cow::Borrowed;
        assert_eq!(
            opts,
            Options {
                user: bs("user."),
                password: bs("pass"),
                schema: bs("schema"),
                host: bs("dbname"),
                query: HashMap::from_iter(vec![("a 1".to_owned(), "b".to_owned())]),
                fragment: bs("frag"),
            }
        );
    }

    #[test]
    fn options_no_schema() {
        let opts = Options::parse_uri("dbname?a#frag").unwrap();
        assert_eq!(
            opts,
            Options {
                user: Default::default(),
                password: Default::default(),
                schema: Default::default(),
                host: Cow::Borrowed("dbname"),
                query: HashMap::from_iter(vec![("a".to_owned(), "".to_owned())]),
                fragment: Cow::Borrowed("frag")
            }
        );
    }

    #[test]
    fn options_round_trip() {
        let opts_str = "schema://user%2F:pass@dbname?a+1=b#frag%2E";
        let opts = Options::parse_uri(opts_str).unwrap();
        assert_eq!(opts.into_uri(), opts_str);
    }
}
