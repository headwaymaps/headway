# Kubernetes

These configs are experimental, and pretty specific to my own needs at this point - expect to edit them for your own cluster.

`chart/` is a Helm chart of every service. `bin/k8s/deploy <build-dir>` renders it
from the build's committed inputs (`.env`, `artifacts.json`, and
each zone's `zone.json`) and deploys it to the lowercased build directory name as
the namespace, e.g. `bin/k8s/deploy builds/Seattle --dev` deploys to `seattle`.

- `--dev` uses the `maps-earth-dev` containers instead of `maps-earth-latest`.
- `--diff` previews against the cluster instead of applying (`helm plugin install https://github.com/databus23/helm-diff`).
- `--restart` rolls every pod, to pick up containers republished to the same tag.

To render without a cluster, as CI does:

```
bin/k8s/values builds/planet | helm template headway k8s/chart --values -
```

## Data volumes

Every service that needs map data mounts a PersistentVolumeClaim whose name
encodes the version of the data it holds in order to speed up redeployment.

### Deploying a new planet version

```sh
bin/update-planet-version                        # bumps builds/planet/.env
bin/build builds/planet
bin/publish-data builds/planet --host <asset-host>
bin/k8s/deploy builds/planet
```

## Deploying a transit rebuild

```sh
bin/build builds/planet otp-graphs
bin/publish-data builds/planet --host <asset-host>
bin/k8s/deploy builds/planet
```

### Reclaiming old volumes

Volumes accumulate: each planet bump and each transit rebuild leaves the previous one behind, which is what makes a rollback instant. Once you're happy with a rollout, collect them:

```sh
bin/k8s/show-volumes planet             # what exists, and whether anything still uses it
bin/k8s/unused-volumes planet           # just the ones nothing references
bin/k8s/unused-volumes planet --delete  # delete them
```

`unused-volumes --delete` deletes any headway-labeled PVC that no Deployment references and no running pod mounts, so it can't remove a volume that's actually in use.
