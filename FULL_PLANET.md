# Full-planet considerations

## Build requirements

Dagger builds can consume significant disk space due to layer caching. For planet builds, you may need to perform builds on a system with deduplication. There are two main options for that: LVM VDO and ZFS. ZFS has been tested for this purpose.

With deduplication enabled the disk space requirement for a full-planet build is fairly low, expect to need around 1TB of fast storage. I've observed deduplication ratios as high as 15x, with an average of 8-10x for full-planet work.

### Dagger

Planet builds may take multiple days, depending on your hardware. Dagger will automatically manage build sessions and timeouts, but you may need to configure your system for long-running builds.

## Runtime requirements

Expect to need 64GB of RAM and fast disk for elasticsearch in particular.

### Kubernetes

TODO
