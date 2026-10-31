# Push numbers and the command line

## Push numbers

A **Push** number has no source to poll. Its value is sent to Cownter by your
own scripts, cron jobs, CI steps or webhook relays. Pushed values are stored
and handled exactly like polled ones: history, today's change, sparkline,
goals, alerts, sounds and formulas all work.

1. Add a number and choose **Push** as the source. Pick whether it shows a
   number or text.
2. Open **General → Push and command line** for the address, the token and a
   ready-to-copy `curl` command.

### Endpoint

Cownter listens on `http://127.0.0.1:47615` (only this computer can reach it).
The port can be changed on the General page. If the port is taken, the page
shows the error and a **Try again** button; the rest of the app keeps running.

Every request needs the token:

```
Authorization: Bearer <token>
```

The token is created on first run and kept in the system keyring (or in
Cownter's database when no keyring is available). **Regenerate** on the
General page replaces it; the old token stops working right away.

`<entry>` is the number's name (case-insensitive), short name or id.

| Request | Body / query | Does |
| --- | --- | --- |
| `POST /v1/set` | `{"entry": "Orders", "value": 42}` | Sets a Push number. For text numbers, `value` is a text. |
| `POST /v1/add` | `{"entry": "Orders", "by": 1}` | Adds to a Push number (`by` defaults to 1, can be negative). |
| `GET /v1/get` | `?entry=Orders` | Returns any number's current value. |
| `GET /v1/list` | | Returns every number. |
| `GET /v1/export` | `?entry=Orders` (optional) | Returns the stored history as CSV, for one number or for all. |

Answers are JSON, except `/v1/export`. A number looks like:

```json
{"id": 3, "name": "Orders", "slug": "orders", "push": true, "value": 42.0, "text": null, "status": "live", "updatedAt": 1793000000000, "deltaToday": 5.0}
```

`/v1/export` answers with `text/csv`: a header line `entry,timestamp,value`,
then one line per stored reading. Timestamps are ISO-8601 in local time with
the offset (`2026-10-31T14:05:00+01:00`). Names that hold a comma, a quote or a
line break are quoted, with quotes doubled.

Errors come back as `{"error": "..."}` with status 400 (bad input), 401 (missing
or wrong token), 404 (no such number), 409 (the number is not a Push number,
or the name matches more than one number) or 405 (wrong method).

Only Push numbers can be set or added to; other numbers get their value from
their source.

### Example

```sh
curl -X POST http://127.0.0.1:47615/v1/set \
  -H 'Authorization: Bearer <token>' \
  -H 'Content-Type: application/json' \
  -d '{"entry":"Orders","value":42}'
```

## Command line

The Cownter binary doubles as a command-line client for the running app:

```sh
cownter set "Orders" 42      # set a Push number (or text)
cownter add "Orders" 1       # add to it; the amount defaults to 1
cownter get "Orders"         # print the value of any number
cownter list                 # every number with its value
cownter export "Orders"      # history of one number as CSV, printed
cownter export --out all.csv # history of every number, written to a file
cownter --help
```

The commands talk to the running app over the local endpoint and read the
port and token from the same place the app keeps them, so there is nothing to
configure. They print `Cownter isn't running` when the app is not open.

`export` prints the CSV unless `--out <file>` is given; `--out` replaces an
existing file. In the app, **Export CSV** on the Numbers page (for all numbers,
or the download button on a row for one) saves
`cownter-<name>-<date>.csv` to your Downloads folder, or your home folder when
there is none, and never replaces a file: it adds `-2`, `-3` instead.

Exit codes: `0` on success, `1` on any error (the message goes to stderr).
