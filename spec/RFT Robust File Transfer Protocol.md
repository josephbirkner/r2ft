# RFT: Robust File Transfer Protocol

by Johannes Abel, Joseph Birkner and Peter Okelmann  
**Date:**    15.05.2020  
**Version:** v0.0.1  
**Status:**  Working Draft


## Requirements

1. Point-to-point operation (one client, one server)
2. The protocol MUST be UDP-based
3. The protocol MUST NOT use other protocols on top of UDP
4. The protocol MUST NOT use another protocol on top of itself
5. The protocol MUST be able to recover from connection drops (e.g., due to outages)
6. The file transfer MUST be “reliable”
7. The protocol MUST support flow control
8. The protocol MUST realize a minimal congestion control (e.g., AIMD)
9. The protocol does not need to support authentication, integrity protection, or encryption
10. The protocol MUST support checksums for the received files

## Request Message Types

### `GET`

This operation allows a client to retrieve a (part of) a file.
A client may specify the `begin` and/or `end` fields. Different
combinations of these fields MUST alter the semantics of the command:

* Neither `begin` nor `end` is specified:
  The server MUST launch an ACK/NACK-based transmission loop of the requested file from start to `EOF`.
* Only `begin` is specified: 
  The server MUST launch an ACK/NACK-based transmission loop of the requested file from `begin` to `EOF`. This is akin to resuming the transmission.
* Both `begin` and `end` are specified:
  The server MUST launch an ACK-free Get-Range-Based transmission loop
  of the requested file from `begin` to `end`.

If `begin` is zero or unspecified, the server MUST send file-specific
METADATA to the client.

If the requested file does not exist, the server MUST respond with ABORTED.

**Behavior when requesting directories:**

If the client sends GET with a directory path, a server MAY either respond with ABORTED if Directory GET is unsupported, or ordered ACK-based transmission of all files in a directory (recursively).

### `LS`

This operation allows a client to list the contents of the root directory at which the server is serving files, or a specific subdirectory.

A server may choose not to implement this operation and reply with ABORTED.

### `ABORT`

This operation allows a client to cancel an ongoing transmission.

## Response Message Types

### `CHUNK`

A chunk of data which belongs to a file at a specific location with a specific SHA. A client MUST use the SHA to verify that it is consistently receiving data
for the same file.

Chunks which belong to the same transmission sequence and the same file
MUST have equal size, except for the last chunk in the sequence, which MAY
be shorter than those preceding it.

### `LIST`

A list of file-names which is sent in response to an LS command.

### `ABORTED`

Tell the client that a specific operation was aborted for a specific reason.

## Framing

Any request/response message MUST consist of the following contents:

```
+---------+-------+------+------+===================+-------+
| version | seqno | type | size | payload           | crc32 |
+---------+-------+------+------+===================+-------+
```

Conventions:

* `version` MUST identify the used released version of this protocol in integer form like `MAJOR*10e6 + MINOR*10e3 + PATCH`
* `seqno` MUST be a fixed-size unsigned integer.
* `type` MUST be a fixed-size unsigned integer. It MUST correspond to a unique constant which represents one of the values listed under *Types* in the Payloads section.
* `size` MUST be a fixed-size unsigned integer. It describes the size of `payload`, in bytes.
* `payload` MUST be a `size`-bytes long byte array.
* `crc32` MUST be a CRC32-hash of the packet, including the version and payload.

When a client receives a packet, it MUST verify that the received packet data
matches the advertised CRC32. If a match exists, a client MUST send an
ACK for the packet at some point. If the CRC does not match, a client
MUST send a NACK for the packet at some point.

## Error Detection

### `ACK`

The ACK message is used to acknowledge that a message was received by the server or client.

### `NACK`

The NACK message is used to inform that a message was NOT correctly received by the server or client. This is detected through a CRC mismatch, or a timeout.

## Payloads

Payloads of the `ACK` amd `NACK` packets MUST be fitted into a single packet. Other payloads MAY be splitted into multiple packets to be assembled again using the sqeuence number.

### Bidirectional

| Type     | Semantics      | Payload      | 
|----------|----------------|--------------|
| `ACK`    | Received all packets up to `seqno`. | - |
| `NACK`   | Packets `seqno` not received, request for retransmission starting from `seqno`. | - |

### Client → Server

| Type     | Semantics      | Payload      | 
|----------|----------------|--------------|
| `GET`    | Get a (part of) a file. | `path` ( `begin` \| `begin` `end` \) |
| `LS`     | List the contents of a directory on the server. | `path` |
| `ABORT`  | Abort the current operation. | - |

### Server → Client

| Type     | Semantics         | Payload      | 
|----------|-------------------|--------------|
| `CHUNK`    | A part of a file. | `path` `sha` `count` `index` `data` |
| `METADATA` | File metadata.  | `path` `sha` `size` `perms` |
| `LIST` | File metadata.  | `count` ( `path` ){`count`} |
| `ABORTED` | The operation was unexpectedly ended. | `reason` |

## Data Types

* `path`: MUST be a string type.
* `sha`: MUST be a fixed-size byte array type.
* `size`: MUST be a positive integer.
* `count`: MUST be a positive integer.
* `index`: MUST be a positive integer LESS THAN it's companion `count`.
* `data`: MUST be a variably-sized byte array.
* `reason`: MUST be one of ( `File not found.` | `File changed/read error.` | `File could not be opened.` | `Aborted by client` | `Version mismatch.` )

## State Machines
![Server state during request processing.](https://nextcloud.pogobanane.de/index.php/s/NdAxFspcaJXXmGE/preview)

## Transmission Modes

### ACK/NACK-Based

All participants MUST send their first packet with a sequence number of one and
increment it for every new packet sent. Retransmitted packets MUST use the same
sequence number as in their original transmission. 


The client and the server MUST keep track of a CWND (Congestion Window) and the
highest sequence number up until which it has received all packets. Initially
the CWND SHALL be ten. After at most CWNG number of packets have been received,
the participant MUST send an ACK (Acknowledgement) Packet. A participant who has
sent CWND packets without receiving an ACK packet MUST wait for such a packet
before sending more. Thereby a sender can be rate limited by the receiver by not sending ACK packets (**Flow Control**). 

When a packet fails the CRC check or an expected packet is not received for a
fixed amount of time which MUST be shorter that the connection timeout, the
participant MUST send a NACK packet with the highest sequence number until which
all packets have been received as sequence number. On receiving a NACK packet a
participant MUST half its CWND (**Congestion Control**). It also MUST retransmit all packets with sequence numbers higher than the one specified in the NACK payload.

After receiving an ACK packet, the participant MAY increase the CWND by the number of acknowledged packets. Furthermore the buffers required for sending the acknowledged packets MAY be freed. 

### GET-Range-Based

A client can use the range start and end parameters to request only the chunk of a file which he is missing. In this mode no retransmission of packets take place. 


### Considerations

From a user view, only successful transmission of the whole file matters. The transfer is not delay critical and timely in-order delivery of data is not required (but may be appreciated for efficiency/scalability reasons). However all data should be transferred reliably.
Disregarding the initial file request, a client may have to transfer information for:

1) Congestion Control
2) Errors
3) Keep-Alive/Heartbeat
4) Re-request lost or corrupted packets

Because a timely in-order delivery of data isn't needed, acknowledgements could mean an unnecessary overhead. ACKs, NACKs and congestion or flow control information should be sent when they actually make sense.
ACKs should be sent

 - when a file was received completely so the server can close it / delete state

NACKs should be sent

 - after a timeout based on
    - when the respective file is otherwise complete 
    - when a specific amount of data was (not) received (or is to be received)

The client should respond with flow control information 

 - after receiving a certain amount of data (fixed, e.g. based on client buffer size)

The client should respond with congestion control information

- after a certain amount of time (fixed, or e.g. based on RTT).

The interpretation of this information can be done either on the server side or the client side, based on the history of received data per time unit.

## Packet Scaling

Request ideal packet size from OS.