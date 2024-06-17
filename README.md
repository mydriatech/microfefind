# microfefind

Micro front end discovery on Kubernetes.


## Summary

`microfefind` enables client side discovery of micro front ends (µFEs) from labeled Kubernetes `Ingress` declarations.

Development teams can add¹/remove/update their services independently of other teams in their own monitored `Namespace` and no longer require access to any other part of the applications infrastructure.

The `microfefind` REST API exposes a list of detected hosts and paths and additional permitted (filtered) annotations from the `Ingress`.

This project is agnostic to the frameworks used for the main front end (FE) or the µFEs.

¹ Assuming that you build in conventions in the FE app to automatically include and position these µFEs.


## Features

* Cloud native building block for micro front end web apps.
* Embed as a side-car container or run separately with a Helm chart.
* Transparently works with any deployment strategy that uses standard Kubernetes mechanism behind the scenes. (Canary, Rolling and Blue/green)
* Open API documented REST API.
* Low resources usage.
* Horizontally scalable.
* Written in Rust to avoid common memory related security exploits.

## Usage

### Installation using Helm

[Helm](https://helm.sh) must be installed to use the charts.  Please refer to
Helm's [documentation](https://helm.sh/docs) to get started.

Once Helm has been set up correctly, add the repo as follows:

    helm repo add <alias> https://mydriatech.github.io/microfefind

If you had already added this repo earlier, run `helm repo update` to retrieve
the latest versions of the packages.  You can then run `helm search repo
<alias>` to see the charts.

Override default settings from the [default values.yaml](charts/microfefind/values.yaml) in `microfefind-values.yaml`.

To install the <chart-name> chart:

    helm upgrade --install --atomic --create-namespace \
        --namespace microfens \
        --values microfefind-values.yaml \
        my-<chart-name> <alias>/<chart-name>

To uninstall the chart:

    helm delete --namespace microfens my-<chart-name>


### Usage notes for main front end team and architects

Dynamic front end discovery can help scale your organization towards continuous delivery (CD), but each client will make network calls proportional to the number of µFEs.
If you are building an app with planet scale audience, where users only use a small subset of the features each time, you might want to reconsider your strategy.

The `Service` pointed to by each `Ingress` path and the `Pod`s matched by the lables on each such `Service`, are monitored for changes as well.
This enables the main FE to detect whenever a newer version of the µFE is available and also supports different release flows like rolling updates, blue/green or canary releases.

To dynamically load/remove µFEs in the main FE app, it needs to poll the `microfefind` API for updates.

OpenAPI documentation is available at `/api/v1/openapi.json`.

Even if this enables decoupling of team releases and enables more agile continuous delivery, you still need to ensure that design and user experience (UX) is coherent for the application.
You also need to establish a contract/convention where µFEs declare what they provide and establish how the in browser message passing between components should be achieved.

To allow a development team to support µFEs for multiple application in the same `Namespace`, change the default label selection `MICROFEFIND_INGRESSFILTER_MATCHLABELS` to include additional qualifying labels like target web app and/or environment. Do  __not__  use this to filter out features based on entitlements or region, since this will only hide exposed services and will not replace authorization checks in each µFE.


### Usage notes for µFE teams

Label the `Ingress` of the Helm chart with `microfe: "true"` (and/or other value as communicated by the main FE team):

```
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  labels:
    microfe: "true"
    microfe/target: webapp1
    microfe/environment: qa
  annotations:
    microfe/custom-annotation: "custom-fe-contract-values.json"
```


## License

[Apache License 2.0 with Free world makers exception 1.0.0](LICENSE-Apache-2.0-with-FWM-Exception-1.0.0)

The intent of this license to

* Allow makers, innovators, integrators and engineers to do what they do best without blockers.
* Give commercial and non-commercial entities in the free world a competitive advantage.
* Support a long-term sustainable business model where no "open core" or "community edition" is ever needed.

## Governance model

This projects uses the [Benevolent Dictator Governance Model](http://oss-watch.ac.uk/resources/benevolentdictatorgovernancemodel).

See also [Code of Conduct](CODE_OF_CONDUCT.md) and [Contributing](CONTRIBUTING.md).
