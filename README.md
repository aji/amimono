# Amimono

Amimono is an experimental, in-progress, library-level modular monolith
framework for Rust, akin to [Aspire](https://aspire.dev/) and the
now-discontinued [Service Weaver](https://serviceweaver.dev/).

> [!WARNING]
> much of what follows is aspirational, as Amimono is very much a work
> in progress. Read it for an idea of what Amimono wants to be, not what it
> currently is.

The *modular monolith* is a relatively new approach to system design which aims
to realize a plausible middle point between a microservices-based architecture
and a monolithic architecture. With Amimono, your application is written and
developed as one monolithic codebase, but its behavior in a production
environment is heterogenous in a way that resembles traditional microservices.

## Features

* **Lightweight** -- Amimono introduces negligible costs to runtime and binary
  size. Amimono's benefits come from the architecture it enables, rather than
  complex algorithms or expensive computations.

* **Safe** -- Sometimes a system will run multiple incompatible versions of an
  application concurrently, such as during a deployment. This is a common source
  of errors if not handled carefully. Amimono prevents this by ensuring that
  components from different revisions do not interact. This means that the
  entire application is built, type-checked, tested, and deployed as one atomic
  unit, giving greater confidence that the running configuration will behave
  as expected.

* **Batteries-included** -- Functionality such as observability, operational
  tooling, etc. is easy to set up. All opt-in, of course.

* **Simple deployments** -- Amimono includes built-in support for deploying
  applications to common distributed runtimes, such as Kubernetes.

* **Flexible** -- It's easy to get started with Amimono's built-in
  API and CLI, but its abstractions can be pried open and used in other ways for
  use cases that don't quite fit. For example, if you would like to manage
  deployment and scheduling yourself, Amimono provides the tools to do so with
  as little work as possible.

* **Cooperative** -- Amimono includes natural integrations with other parts of
  the Rust ecosystem, such as `tower` and `hyper`, making it feasible to use in
  existing codebases.

* **Fast local RPC** -- When using Amimono's `Rpc` trait, calls to components
  within the same job become local procedure calls, providing the speed of a
  local

## Concepts

### Component

*Components* are the core abstraction of Amimono. They form the *modular* part
of Amimono's approach to modular monoliths. They are similar in spirit to
processes or services, but are more lightweight. Any async function can be made
into a component, but Amimono provides templates for common types of component
such as RPC components. These templates are a key part of how Amimono simplifies
distributed application development.

Each component has a *label*, which is a static, globally-unique identifier
within the application for that component.

A component can also specify a *binding*, which is a type of network resource
that should be allocated for the component, such as an HTTP binding. Components
can query the Amimono runtime for information about another component's external
binding, via that component's label. For example, in the case of an HTTP
binding, the returned information will be a base URL that can be used to reach
the other component. Similarly, a component can request information about its
internal binding, which for an HTTP binding would be a socket address on which
to start an HTTP server. Allocation and discovery of bindings is a core part of
Amimono's functionality.

A special type of binding is a *local binding*, which can be used to allow
colocated components to communicate directly. A local binding is essentially a
function call, and for example is used by `amimono::rpc` to handle RPC calls
between components in the same process.

### Jobs

A *job* is a collection of components, possibly a single component. In a
practical sense, jobs represent processes. When control is passed to
`amimono::start`, the library inspects the `AMIMONO_JOB` environment variable to
determine which job it's supposed to run, and starts the appropriate components.
The idea of a single binary that has multiple distinct behaviors it can select
between is analogous to things like `busybox`, and `AMIMONO_JOB` is the
mechanism used by Amimono for selecting behaviors.

### Applications

An *application* is a collection of jobs, as well as placement information such
as number of replicas and storage constraints. An application, defined by an
`AppConfig`, is the main information passed to `amimono::start`. In production,
the `AppConfig` is simply used by the job launcher to determine which components
to start for a particular job. The Amimono CLI also uses the `AppConfig` to
allocate bindings.

## Manual deployment

If the deployment configuration achievable with the Amimono CLI is insufficient,
you can choose to build, deploy, and run the application yourself. When doing
so, you will need to start the binary with the following environment variables:

* `AMIMONO_JOB` -- This is a job label that determines which job in the
  `AppConfig` to start.

* `AMIMONO_BINDINGS` -- This is a path to a TOML file containing binding
  allocations. You can manage this file by hand, or generate it at build time
  from the `AppConfig`.

By running the compiled binary with `AMIMONO_JOB="_config"`, the `AppConfig`
will be dumped as JSON, allowing a tool to process it to generate bindings.