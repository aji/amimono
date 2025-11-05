# amimono

amimono is an experimental, in-progress, library-level modular monolith
framework for Rust, akin to the now-discontinued [Service
Weaver](https://serviceweaver.dev/).

> NOTE: much of what follows is aspirational, as amimono is very much a work
> in progress. Read it for an idea of what amimono wants to be, not what it
> currently is.

The *modular monolith* is a relatively new approach to system design which aims
to realize a plausible middle point between a microservices-based architecture
and a monolithic architecture. With amimono, your application is written and
developed as one monolithic codebase, but its behavior in a production
environment is heterogenous in a way that resembles a collection of
microservices.

Additionally, since key aspects of the distributed architecture are visible to
amimono, a wide variety of operationally-relevant tooling such as
infrastructure, deployment, maintenance, and observability can be handled
partially or fully by the library.

## Concepts

### Component

*Components* are the core abstraction of amimono. They form the *modular* part
of amimono's approach to modular monoliths. They are similar in spirit to
processes or services, but have a stronger typology that allows amimono to
provide a useful means of interacting with and managing them. For example,
amimono allows your application to define *RPC components*. These are components
that serve requests in an RPC fashion, and other components can make requests to
them using amimono's infrastructure.

In a production environment, every node has the same binary deployed to it.
However, when the process starts and control is passed to amimono, only a
specific subset of components is activated, possibly only one component. This is
how amimono achieves the heterogenous behaviors characteristic of a
microservices-based architecture.

### Applications and placements

Every system built with amimono requires an *application* be defined for it. The
application represents the global information about which components exist, how
many replicas are defined for them, etc. Specifically, the application defines
the *placement* for each component, which in particular means defining how many
replicas are required, which storage resources the component expects, etc.

It's possible for a single amimono codebase to have multiple applications
defined for it, or for the application's definition to vary based on the
environment. For example, you might have an application definition targeting a
kubernetes cluster, but vary the definition based on whether the target cluster
is production or pre-production. However, for all nodes participating in a
single deployment of an amimono system, the application definition is fixed.

The application definition must be reproducible at each node. This is because
nodes require the application definition for discovery, even nodes running only
a single component.