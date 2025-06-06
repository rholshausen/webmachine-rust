

# 0.6.1 - Small fixes

* 654cb97 - feat: Add Server-Timing response header (Ronald Holshausen, Thu May 15 15:49:22 2025 +1000)
* 7749919 - chore: Add incoming request and dispatch debug statements (Ronald Holshausen, Thu May 15 15:05:08 2025 +1000)
* bd62fa3 - feat: Add convenience method to downcast a metadata anything value (Ronald Holshausen, Thu May 15 11:17:55 2025 +1000)
* 2cfa560 - chore: do not dump the full response on every request (Ronald Holshausen, Thu May 15 10:14:08 2025 +1000)
* a316132 - fix: Support wild card with acceptable_content_types (Ronald Holshausen, Thu May 15 10:04:35 2025 +1000)
* 5989849 - bump version to 0.6.1 (Ronald Holshausen, Mon May 12 10:53:50 2025 +1000)

# 0.6.0 - Support async resource methods + resource trait

* fb55351 - chore: Update release script (Ronald Holshausen, Mon May 12 10:49:50 2025 +1000)
* 0d9bc0c - chore: Update macos runners to latest (Ronald Holshausen, Mon May 12 09:27:01 2025 +1000)
* d4e99be - feat: Add support to store anything the running context (Ronald Holshausen, Fri May 9 16:39:31 2025 +1000)
* 8370e01 - feat: Make resource_exists async as it may need to access async resources (Ronald Holshausen, Fri May 9 16:39:01 2025 +1000)
* cc81cb9 - feat: Update Dispatcher to use the Resource trait (Ronald Holshausen, Fri May 9 13:20:07 2025 +1000)
* 3a8879e - feat: Add a Resource trait for Webmachine resources (Ronald Holshausen, Fri May 9 12:20:04 2025 +1000)
* eab3bf4 - feat: Support variables on resource paths (Ronald Holshausen, Thu May 8 17:41:36 2025 +1000)
* f9929bf - fix: When comparing content-type values, ignore the attributes (Ronald Holshausen, Tue May 6 15:24:03 2025 +1000)
* c17fb37 - feat: Make process_post async (Ronald Holshausen, Tue May 6 09:23:52 2025 +1000)
* 18d9ef4 - feat: Make finalise_response, dispatch_to_resource and render_response functions async (Ronald Holshausen, Mon May 5 16:05:48 2025 +1000)
* 3107974 - chore: Update to 2024 Rust edition (Ronald Holshausen, Mon May 5 10:43:07 2025 +1000)
* f2157a6 - bump version to 0.5.1 (Ronald Holshausen, Fri Jun 28 15:14:23 2024 +1000)

# 0.5.0 - Refactor: simplify things by removing all references, lifetimes, arcs and mutexes

* c43fb7d - feat: Implement Default and From for MetaDataValue (Ronald Holshausen, Fri Jun 28 15:10:50 2024 +1000)
* 7567ef0 - Refactor: simplify things by removing all references, lifetimes, arcs and mutexes (Ronald Holshausen, Fri Jun 28 14:24:58 2024 +1000)
* b1e9ba2 - chore: Bump minor version (Ronald Holshausen, Fri Jun 28 12:45:03 2024 +1000)
* dc2936f - bump version to 0.4.1 (Ronald Holshausen, Fri Jun 7 15:55:59 2024 +1000)

# 0.4.0 - Upgrade Hyper and Http crates to 1.0

* 6ed86b0 - chore: Update readme (Ronald Holshausen, Fri Jun 7 12:49:51 2024 +1000)
* d90b39c - chore: Convert to Hyper and Http 1.0 (Ronald Holshausen, Fri Jun 7 12:46:15 2024 +1000)
* d1c0015 - chore: Update deps, setup GH build (Ronald Holshausen, Fri Jun 7 11:48:58 2024 +1000)
* 8ed0beb - bump version to 0.3.1 (Ronald Holshausen, Wed Jun 14 10:33:28 2023 +1000)

# 0.3.0 - update to Rust 2021 edition + switch to using the tracing crate

* 0b9c91d - chore: switch to using the tracing crate (Ronald Holshausen, Wed Jun 14 10:19:49 2023 +1000)
* e5b7dcf - chore: Bump version and update to Rust 2021 edition (Ronald Holshausen, Wed Jun 14 10:05:08 2023 +1000)
* 6cafea1 - bump version to 0.2.3 (Ronald Holshausen, Mon Jan 4 11:20:40 2021 +1100)

# 0.2.2 - Update crates to latest

* 3d3cfb1 - chore: upgrade crates to latest (including hyper to 0.14) (Ronald Holshausen, Mon Jan 4 11:16:04 2021 +1100)
* 1dadfa6 - chore: fix changelog (Ronald Holshausen, Mon Sep 28 10:13:56 2020 +1000)
* a411502 - bump version to 0.2.2 (Ronald Holshausen, Mon Sep 28 10:13:34 2020 +1000)

# 0.2.1 - Small enhancements

* 0d44f1d - chore: record reasons for transitions in the state machine (Ronald Holshausen, Sun Sep 27 18:11:30 2020 +1000)
* f3968da - fix: OPTIONS response should be a 204 (Ronald Holshausen, Sun Sep 27 17:28:06 2020 +1000)
* d0cae8a - chore: log out state machine execution at trace level (Ronald Holshausen, Sun Sep 27 17:26:03 2020 +1000)
* 67d440c - fix: correct spelling (Ronald Holshausen, Sun Sep 27 17:22:38 2020 +1000)
* ae705c8 - chore: remove lazy_static from example; optmise default callbacks (Ronald Holshausen, Sun Sep 27 17:21:12 2020 +1000)
* 3adf5d8 - bump version to 0.2.1 (Ronald Holshausen, Sun Sep 27 09:34:46 2020 +1000)

# 0.2.0 - Thread-safe async version based on Hyper 0.13

* 07020b2 - chore: bump version (Ronald Holshausen, Sat Sep 26 16:30:07 2020 +1000)
* a4b4535 - chore: cleanup warnings (Ronald Holshausen, Sat Sep 26 16:29:30 2020 +1000)
* edc9b80 - feat: pass the resource to all callbacks (Ronald Holshausen, Sat Sep 26 16:27:15 2020 +1000)
* 5a0554f - chore: update rust docs (Ronald Holshausen, Sat Sep 26 16:13:54 2020 +1000)
* bec99a8 - feat: support callbacks usable across threads (Ronald Holshausen, Sat Sep 26 15:18:02 2020 +1000)
* ac10295 - bump version to 0.1.1 (Ronald Holshausen, Thu Sep 24 12:30:11 2020 +1000)

# 0.1.0 - Update to Rust 2018 + updated crates + handle query parameters

* 4bc1048 - feat: query parameter support was missing (Ronald Holshausen, Thu Sep 24 11:58:13 2020 +1000)
* 1a4efc6 - chore: update crates to latest (Ronald Holshausen, Thu Sep 24 11:23:34 2020 +1000)
* 9dd4ae8 - chore: upgrade to Rust 2018 (Ronald Holshausen, Thu Sep 24 11:14:04 2020 +1000)
* 57e6daf - fixed: doc test (Ronald Holshausen, Sun Aug 11 17:44:26 2019 +1000)
* 001a67d - chore: drop hyper and use http crate (Ronald Holshausen, Sun Aug 11 17:16:59 2019 +1000)
* 1a4cba8 - chore: bump version (Ronald Holshausen, Sun Aug 11 16:15:35 2019 +1000)
* 52a7914 - chore: cleanup imports (Ronald Holshausen, Sun Aug 11 16:14:48 2019 +1000)
* dc5139b - fix: replace ristc_serialize with serde (Ronald Holshausen, Sun Aug 11 16:03:27 2019 +1000)
* 0204219 - chore: upgrade crates (Ronald Holshausen, Sun Aug 11 15:53:56 2019 +1000)
* a907fd9 - Revert "Upgraded hyper to 0.11 and started converting Webmachine to a trait + future based implementation" (Ronald Holshausen, Sun Aug 11 15:41:47 2019 +1000)
* f23dc71 - chore: added changelog and release script (Ronald Holshausen, Sun Aug 11 15:24:29 2019 +1000)
* 9c21687 - Upgraded hyper to 0.11 and started converting Webmachine to a trait + future based implementation (Ronald Holshausen, Sat Nov 4 12:33:47 2017 +1100)

# 0.0.0 - Initial Version

* ed8ee7c - add build status badge back to readme (Ronald Holshausen, Sun Jun 26 13:07:41 2016 +1000)
* b74e643 - update cargo manifest with doc and repo urls (Ronald Holshausen, Sun Jun 26 13:05:40 2016 +1000)
* 1d82a32 - reformat the doc examples (Ronald Holshausen, Sun Jun 26 12:52:38 2016 +1000)
* c972277 - added readme docs to module docs (Ronald Holshausen, Sun Jun 26 12:44:37 2016 +1000)
* 64e23b4 - small correction to readme (Ronald Holshausen, Sun Jun 26 12:10:04 2016 +1000)
* a5daf12 - update the readme (Ronald Holshausen, Sun Jun 26 12:06:03 2016 +1000)
* 8fc971f - use Arc instead of RC so the resources can be accessed by different threads (Ronald Holshausen, Fri Jun 24 14:50:39 2016 +1000)
* 23d1cec - webmachine dispatcher needs to be thread safe (Ronald Holshausen, Fri Jun 24 14:30:26 2016 +1000)
* 22549e3 - read the hyper request body into a webmachine request body (Ronald Holshausen, Thu Jun 23 12:20:49 2016 +1000)
* f536e09 - if result is 200 and a GET, write a body (Ronald Holshausen, Thu Jun 23 11:25:40 2016 +1000)
* 338b605 - write out the body of the response if there is one (Ronald Holshausen, Thu Jun 23 10:51:31 2016 +1000)
* 29f99db - implemented adding final response headers (Ronald Holshausen, Wed Jun 22 17:04:54 2016 +1000)
* 17e9b67 - completed the final steps in the state machine (Ronald Holshausen, Wed Jun 22 15:45:23 2016 +1000)
* 4f02445 - implemented handling of posts where the result is a redirect (Ronald Holshausen, Wed Jun 22 13:49:13 2016 +1000)
* e345f56 - allow the delete action to return a status code if the deletion fails (Ronald Holshausen, Wed Jun 22 11:40:53 2016 +1000)
* b18791c - implemented handling of delete (Ronald Holshausen, Tue Jun 21 21:30:41 2016 +1000)
* d6b237f - implemented if modified since check (Ronald Holshausen, Tue Jun 21 21:13:42 2016 +1000)
* 8c822df - Implemented etag in If-None-Match check (Ronald Holshausen, Tue Jun 21 17:32:07 2016 +1000)
* 217cccc - implemented If-None-Match * check (Ronald Holshausen, Tue Jun 21 17:16:11 2016 +1000)
* 5e5587a - implemented the if unmodified since check (Ronald Holshausen, Tue Jun 21 16:53:59 2016 +1000)
* 36c926c - implemented If-Match etag check (Ronald Holshausen, Mon Jun 20 16:23:45 2016 +1000)
* 0f63a66 - implemented moved resources and posts to missing resource checks (Ronald Holshausen, Sun Jun 19 14:25:51 2016 +1000)
* 5915abb - implemented conflict and allow missing post checks (Ronald Holshausen, Sun Jun 19 13:45:53 2016 +1000)
* 63482f4 - implemented the moved permanently check (Ronald Holshausen, Sun Jun 19 10:48:26 2016 +1000)
* dc38aeb - first run through the state machine to a 404 response (Ron Holshausen, Thu Jun 16 17:29:31 2016 +1000)
* 0069072 - implemented the resource missing with If-Match = * check (Ron Holshausen, Thu Jun 16 16:58:00 2016 +1000)
* 8955e0a - implemented the accept encoding check (Ron Holshausen, Thu Jun 16 15:13:48 2016 +1000)
* 9feff38 - implemented charset check (Ron Holshausen, Thu Jun 16 12:05:31 2016 +1000)
* c8320a3 - implemented content language check (Ron Holshausen, Wed Jun 15 15:40:17 2016 +1000)
* ea83571 - implemented the acceptable media type check (Ron Holshausen, Tue Jun 14 16:30:44 2016 +1000)
* 3912dae - implemented handling of options requests (Ron Holshausen, Sun Jun 12 18:56:56 2016 +1000)
* 7df8e35 - implemented the content type check (Ron Holshausen, Sun Jun 12 17:15:32 2016 +1000)
* 4ca9c76 - implemented the authorised and forbidden checks (Ron Holshausen, Sun Jun 12 11:37:34 2016 +1000)
* 4cce2be - implemented malformed request check (Ron Holshausen, Sun Jun 12 11:18:58 2016 +1000)
* edac4b3 - implemented uri too long and method allowed checks (Ron Holshausen, Sat Jun 11 23:17:39 2016 +1000)
* d8325a1 - implemented the known method check (Ron Holshausen, Sat Jun 11 22:12:32 2016 +1000)
* f9dbf98 - implemented the available callback (Ron Holshausen, Sat Jun 11 20:58:00 2016 +1000)
* 2c2942a - basic state machine implemented (Ron Holshausen, Sat Jun 11 16:06:27 2016 +1000)
* 62bd3e3 - added travis badge to readme (Ronald Holshausen, Thu Jun 9 22:18:46 2016 +1000)
* 70a9bcd - added travis build (Ronald Holshausen, Thu Jun 9 22:16:06 2016 +1000)
* 79643f0 - add the basic structs and support libraries (Ronald Holshausen, Thu Jun 9 22:12:28 2016 +1000)
* 905d4a6 - add rust skeleton (Ronald Holshausen, Wed Jun 8 22:10:55 2016 +1000)
* e25cab5 - Initial commit (Ronald Holshausen, Wed Jun 8 22:04:36 2016 +1000)
