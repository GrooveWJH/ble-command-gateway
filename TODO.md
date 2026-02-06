# TODO

## Heartbeat (Client <-> Server)

- [ ] Add a dedicated low-level heartbeat channel that does not interfere with command traffic.
- [ ] Add heartbeat GATT characteristics (separate from command read/write): client sends heartbeat, server replies ack.
- [ ] Heartbeat cadence: client sends every 1s.
- [ ] Timeout rule: single heartbeat >3s without ack counts as timeout.
- [ ] Disconnect rule: 5 consecutive heartbeat timeouts => mark link disconnected.
- [ ] Keep heartbeat fully isolated from command protocol parsing/queues.
- [ ] Add server-side lightweight heartbeat logs (recv/ack/last seen).
- [ ] Add client-side session health monitor and reconnect hint on disconnect.
- [ ] Refactor interactive client runtime to support true background heartbeat task (async session loop).
- [ ] Wire menu connection lamp to heartbeat session status (green=alive, red=disconnected).
