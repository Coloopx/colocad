# Notices

Colocad and the included Agent CAD application code are distributed under the MIT license. The Rust application builds against SolidCore at revision bff1569a4c3347ed9af67faab71578cadf84fde2. SolidCore includes work derived from NVIDIA USD BRep / SMLib. The kernel's Apache-2.0 license, notices, attribution and licenses of Rust dependencies ship in `kernel/licenses`. Source provenance is also recorded in `third_party`. This source candidate applies Colocad changes for planar Face 37 containment and spline property analysis. The upstream sources, patches, and resulting files are hash-pinned in `third_party/solidcore.lock.json`.

The session catalog executable uses rusqlite and bundled SQLite. Rust dependency licenses ship in `kernel/licenses`.

The .NET author references SOLIDWORKS 2024 interop APIs. SOLIDWORKS is a trademark of its respective owner. A local licensed installation provides the proprietary COM application and its interop assemblies. The package contains Colocad's own managed assembly and source in the repository. The experimental file-format investigation uses the independently licensed cadmpeg project at d77e2dd2a9d154e7b432afb022c147864f2618bb.

The browser viewer bundles Three.js under the MIT license; its license is included in `viewer/THREE-LICENSE`. HIWIN and product names in the actuator reconstruction identify the source product; this project is not affiliated with or endorsed by the manufacturer. No manufacturer CAD or catalog PDF is redistributed.
