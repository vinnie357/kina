# Unreal Tournament 99 Demo

Runs an Unreal Tournament 99 dedicated server (OldUnreal v469 patch) and
exposes UDP port 7777 via UDPRoute (Gateway API) or TransportServer (nginx-ingress).

Image: `phasecorex/ut99-server` (public, available for linux/amd64; runs under
Rosetta emulation on Apple silicon nodes)

## arm64 Notes

`phasecorex/ut99-server` is built for amd64. It runs on arm64 Apple Container
nodes via QEMU/Rosetta emulation transparently — no image change needed.
Expect higher CPU usage than native arm64 images due to emulation.

The server binary requires two libraries not present in the base image:

- `libsdl2-2.0-0`
- `libasound2t64`

The manifests install these at container startup via `apt-get`. This adds
~10-15 seconds to first boot.

An `emptyDir` volume is mounted at `/data` to give the server writable space.
Map data is not persisted across restarts.

## Prerequisites

- Running kina cluster with `--cni cilium`
- Either `kina install nginx-gateway-fabric` (Gateway API) or
  `kina install nginx-ingress` (nginx-ingress TransportServer)
- For Gateway API: experimental CRDs + kina-43 RBAC + udp-game listener
- **UT99 client**: Install [OldUnreal UT99 v469](https://github.com/OldUnreal/UnrealTournamentPatches/releases)
  on your Mac for the best compatibility with the v469 server

## Deploy

```bash
# Gateway API variant
kubectl apply -f gatewayapi.yaml

# nginx-ingress variant
kubectl apply -f ingress.yaml
```

## Connect

Start the UT99 client. In the server browser or console:

```
open <NODE_IP>
```

Or use the in-game console (`~`) and type:

```
open <NODE_IP>:7777
```

Wait ~30-60 seconds for the server to finish loading maps before connecting.

Watch server logs:

```bash
kubectl logs -n games deploy/ut99 -f
```

## Teardown

```bash
kubectl delete namespace games
```

(If you also have minecraft in the `games` namespace, delete just the ut99
resources: `kubectl delete -f gatewayapi.yaml`)

## Notes

- Map files are bundled in the image. No external download needed.
- The server starts in `DM-Deck16` by default. Edit `SERVER_PARAMS` env var
  to change the starting map or game type.
- UDP routing requires the experimental Gateway API CRDs and the NGF
  experimental features flag. See `examples/gatewayapi/README.md`.
