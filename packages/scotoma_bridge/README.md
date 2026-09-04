# SCOTOMA production bridge

This package is the only Dart/Rust bridge linked by the Android and web
applications. Its generated API contains session setup, transport, chat and
terminal lifecycle operations only.

The historical compatibility harness and all benchmark/fault-injection APIs
remain isolated under `spikes/client_bridge`; the app does not depend on them.
