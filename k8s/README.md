# Kubernetes

These configs are experimental, and pretty specific to my own needs at this point - expect to edit them for your own cluster.

- `_template/` — the manifests, as `envsubst` templates. This is what you edit.
- `configs/<namespace>/` — rendered output, checked in. Regenerate it rather than editing it by hand.

`bin/k8s/generate <build-dir> <output-dir>` renders one config dir. `bin/k8s/regenerate-all` re-renders all the checked-in ones, which is usually what you want after editing a template.

## Data volumes

Every service that needs map data mounts a PersistentVolumeClaim whose name
encodes the version of the data it holds in order to speed up redeployment.

### Deploying a new planet version

```sh
bin/update-planet-version                        # bumps builds/planet/.env
bin/build builds/planet
bin/publish-data builds/planet --host <asset-host>
bin/k8s/generate builds/planet k8s/configs/planet
kubectl apply -f k8s/configs/planet
```

## Deploying a transit rebuild

```sh
bin/build-transit builds/planet
bin/publish-data builds/planet --host <asset-host>
bin/k8s/generate builds/planet k8s/configs/planet
kubectl apply -f k8s/configs/planet
```

### Reclaiming old volumes

Volumes accumulate: each planet bump and each transit rebuild leaves the previous one behind, which is what makes a rollback instant. Once you're happy with a rollout, collect them:

```sh
bin/k8s/show-volumes planet             # what exists, and whether anything still uses it
bin/k8s/unused-volumes planet           # just the ones nothing references
bin/k8s/unused-volumes planet --delete  # delete them
```

`unused-volumes --delete` deletes any headway-labeled PVC that no Deployment references and no running pod mounts, so it can't remove a volume that's actually in use.

