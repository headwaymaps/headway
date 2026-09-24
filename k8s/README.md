# Kubernetes

These configs are experimental, and pretty specific to my own needs at this point - expect to edit them for your own cluster.

- `_template/` — the manifests, as `envsubst` templates. This is what you edit.
- `builds/<build>/k8s/<latest|dev>/` — rendered output variants, checked in next to the build they were rendered from. Regenerate them rather than editing them by hand.

`bin/k8s/generate <build-dir> <variant>` renders one `latest` or `dev` variant into `<build-dir>/k8s/<variant>`. `bin/k8s/regenerate-all` re-renders all checked-in variants, which is usually what you want after editing a template.

The deploy scripts take a build directory and use its lowercased basename as the Kubernetes namespace. They select the `latest` rendered variant by default; `--dev` selects `dev`. For example: `bin/k8s/apply-latest builds/Seattle --dev` deploys `builds/Seattle/k8s/dev` to the `seattle` namespace.

## Data volumes

Every service that needs map data mounts a PersistentVolumeClaim whose name
encodes the version of the data it holds in order to speed up redeployment.

### Deploying a new planet version

```sh
bin/update-planet-version                        # bumps builds/planet/.env
bin/build builds/planet
bin/publish-data builds/planet --host <asset-host>
bin/k8s/generate builds/planet latest
bin/k8s/apply-latest builds/planet
```

## Deploying a transit rebuild

```sh
bin/build-transit builds/planet
bin/publish-data builds/planet --host <asset-host>
bin/k8s/generate builds/planet latest
bin/k8s/apply-latest builds/planet
```

### Reclaiming old volumes

Volumes accumulate: each planet bump and each transit rebuild leaves the previous one behind, which is what makes a rollback instant. Once you're happy with a rollout, collect them:

```sh
bin/k8s/show-volumes planet             # what exists, and whether anything still uses it
bin/k8s/unused-volumes planet           # just the ones nothing references
bin/k8s/unused-volumes planet --delete  # delete them
```

`unused-volumes --delete` deletes any headway-labeled PVC that no Deployment references and no running pod mounts, so it can't remove a volume that's actually in use.
