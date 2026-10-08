---
name: lidb-linux-networking
description: Linux sockets, netlink, namespaces, routes, nftables and safe diagnostics.
---
# Linux networking skill

**Use when:** implementing network state, tracing, namespace topology or DNS/connection diagnostics.

- Understand direction-dependent Netfilter hooks; avoid fixed-stage depictions of conntrack, nftables and routing.
- Prefer netlink/rtnetlink and documented socket APIs over scraping command output; fallback collectors explicitly label limitations.
- Distinguish namespace identity, interface ifindex, veth peer, bridge and route table/policy routing context.
- Reconcile sockets/process attribution races, ephemeral interfaces, IPv4/IPv6 and dual-stack behavior.
- Explain kernel feature/permission gating; no automatic firewall, route, tunnel or NIC configuration.
- Validate in disposable namespace/veth fixtures only with approved privileges; capture exact commands and cleanup.
- Source reference: [kernel model](../../../docs/ARCHITECTURE.md); external vendor/kernel docs should be checked before implementation.
