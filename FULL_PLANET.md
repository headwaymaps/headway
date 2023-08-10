# Full-planet considerations

## Build requirements

Earthly makes many needless copies of data in its cache, so unless you have truly excessive amounts of storage you will need to perform planet builds on a system with deduplication. There are two main options for that that I'm aware of: LVM VDO and ZFS. I've only attempted to use ZFS for this purpose.

With deduplication enabled the disk space requirement for a full-planet build is fairly low, expect to need around 1TB of fast storage. You'll also need to disable BuildKit parallelism in earthly and set `cache_size_mb` to 10000000 to trick it into using more disk space than you technically have available. I've observed deduplication ratios as high as 15x, with an average of 8-10x for full-planet work.

### Earthly

Earthly added a timeout to buildkit (default 24h). Planet builds may take multiple days, depending on your hardware.
You'll want to increase it, or you can disable it altogether in your `~/.earthly/config.yml`

```
global:
    buildkit_additional_args: ['-e', 'BUILDKIT_SESSION_TIMEOUT=0']
```

## Runtime requirements

Expect to need 64GB of RAM and fast disk for elasticsearch in particular.

### Kubernetes

TODO
