use std::collections::HashMap;

use http::Uri;

use crate::config::RouteConfig;

/// Match a request path to the best matching route
///
/// Uses longest-prefix-first matching:
/// - "/api/users" matches "/api" before "/"
/// - Exact matches take precedence
/// - Returns None if no route matches
///
/// Path normalization rules:
/// - Uses http::Uri for proper canonicalization
/// - Multiple slashes collapsed: "///" -> "/"
/// - Trailing slash ALWAYS removed: "/api/" -> "/api"
/// - Root "/" is the ONLY exception (kept as "/")
/// - Query strings and fragments stripped
pub fn match_route<'a>(
    path: &str,
    routes: &'a HashMap<String, RouteConfig>,
) -> Option<(&'a str, &'a RouteConfig)> {
    // Normalize path using http::Uri
    let normalized = normalize_path(path);

    // Find all matching routes (path starts with route prefix)
    let mut matches: Vec<_> = routes
        .iter()
        .filter(|(route_path, _)| normalized.starts_with(route_path.as_str()))
        .collect();

    // Sort by path length (descending) to get longest match first
    matches.sort_by_key(|(route_path, _)| std::cmp::Reverse(route_path.len()));

    // Return the longest matching route
    matches
        .first()
        .map(|(route_path, config)| (route_path.as_str(), *config))
}

/// Normalize a request path
///
/// Rules:
/// - Uses http::Uri for canonicalization
/// - Strip query string and fragment
/// - Collapse multiple slashes: "///" -> "/"
/// - Remove trailing slash EXCEPT for root "/"
/// - "/api/" becomes "/api" (resource, not directory)
/// - "/" stays as "/" (root resource)
fn normalize_path(path: &str) -> String {
    // Parse as URI to get proper path normalization
    let uri = match path.parse::<Uri>() {
        Ok(uri) => uri,
        Err(_) => {
            // Fallback to manual normalization if parsing fails
            return manual_normalize(path);
        }
    };

    // Get path component (already normalized by http::Uri)
    let path = uri.path();

    // Root is special case - keep as "/"
    if path == "/" {
        return "/".to_string();
    }

    // Remove trailing slash for all other paths (resources, not directories)
    path.trim_end_matches('/').to_string()
}

/// Fallback manual normalization if URI parsing fails
fn manual_normalize(path: &str) -> String {
    // Strip query and fragment
    let path = path
        .split('?')
        .next()
        .unwrap_or(path)
        .split('#')
        .next()
        .unwrap_or(path);

    // Collapse multiple slashes and split into segments
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Reconstruct path
    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}
