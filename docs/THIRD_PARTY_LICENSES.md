# Third-Party Licenses and Software Bill of Materials (SBOM)

LiDB and LiDashBoard are licensed under the [MIT License](../LICENSE).

This document provides a comprehensive inventory of all third-party open-source software libraries compiled into the official release binaries of `lidash` (`lidb-x86_64` and `lidb-aarch64`).

## License compliance statement

All direct and transitive dependencies compiled into LiDB are distributed under permissive, open-source licenses compatible with the MIT license:
- **MIT License**
- **Apache License, Version 2.0**
- **Dual MIT / Apache-2.0**
- **Apache-2.0 with LLVM Exception**

No copyleft (GPL, AGPL, LGPL), non-commercial, or proprietary code is linked into LiDB binaries.

## Dependency inventory

| Crate | Version | License | Primary Purpose | Repository |
| --- | --- | --- | --- | --- |
| `serde` | 1.0.229 | MIT OR Apache-2.0 | Generic serialization framework | https://github.com/serde-rs/serde |
| `serde_json` | 1.0.151 | MIT OR Apache-2.0 | JSON serialization for schema 0.1 | https://github.com/serde-rs/json |
| `serde_derive` | 1.0.229 | MIT OR Apache-2.0 | Derive macro for Serialize/Deserialize | https://github.com/serde-rs/serde |
| `ratatui` | 0.29.0 | MIT | Terminal user interface rendering engine | https://github.com/ratatui/ratatui |
| `crossterm` | 0.28.1 | MIT | Cross-platform terminal raw mode and event loop | https://github.com/crossterm-rs/crossterm |
| `rustix` | 0.38.44 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Safe POSIX/Linux syscall wrappers | https://github.com/bytecodealliance/rustix |
| `linux-raw-sys` | 0.4.15 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Raw Linux kernel syscall definitions | https://github.com/sunfishcode/linux-raw-sys |
| `libc` | 0.2.190 | MIT OR Apache-2.0 | Standard C library type bindings | https://github.com/rust-lang/libc |
| `signal-hook` | 0.3.18 | Apache-2.0 OR MIT | Safe Unix signal handling (SIGINT/SIGTERM) | https://github.com/vorner/signal-hook |
| `signal-hook-mio` | 0.2.5 | Apache-2.0 OR MIT | Mio adapter for Unix signal handling | https://github.com/vorner/signal-hook |
| `mio` | 1.2.4 | MIT | Low-level event-driven I/O | https://github.com/tokio-rs/mio |
| `compact_str` | 0.8.2 | MIT | Inlined small string optimization | https://github.com/ParkMyCar/compact_str |
| `unicode-width` | 0.2.0 | MIT OR Apache-2.0 | Unicode grapheme character display width | https://github.com/unicode-rs/unicode-width |
| `unicode-segmentation` | 1.12.0 | MIT OR Apache-2.0 | Word and grapheme boundary segmentation | https://github.com/unicode-rs/unicode-segmentation |
| `unicode-truncate` | 1.1.0 | MIT OR Apache-2.0 | Unicode grapheme string truncation | https://github.com/Aetf/unicode-truncate |
| `itertools` | 0.13.0 | MIT OR Apache-2.0 | Iterator combinators | https://github.com/rust-itertools/itertools |
| `cassowary` | 0.3.0 | MIT / Apache-2.0 | Constraint-solving engine for TUI layout | https://github.com/derekdreery/cassowary-rs |
| `bitflags` | 2.13.2 | MIT OR Apache-2.0 | Strongly-typed bitflags implementation | https://github.com/bitflags/bitflags |
| `lru` | 0.12.5 | MIT | Fixed-capacity LRU cache | https://github.com/jeromefroe/lru-rs |
| `strum` | 0.26.3 | MIT | Enum iteration and display utilities | https://github.com/Peternator7/strum |
| `paste` | 1.0.15 | MIT OR Apache-2.0 | Identifier macro pasting | https://github.com/dtolnay/paste |
| `indoc` | 2.0.8 | MIT OR Apache-2.0 | Indented multiline string literals | https://github.com/dtolnay/indoc |

## Common license texts

### MIT License

```text
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### Apache License, Version 2.0

Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at:
http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
