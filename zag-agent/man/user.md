# zag user

Manage user accounts on the zag server.

## Synopsis

    zag user <subcommand> [options]

## Description

Manages user accounts used for authenticated `zag connect` sessions served by `zag serve`. Each user has a username, password, and a home directory that constrains the filesystem root the user sees once connected. When a remote client authenticates with `--username` / `--password`, their zag commands are proxied to the server and jailed to that home directory.

Passwords are hashed on disk; plaintext passwords are never stored. If `--password` is omitted, it is prompted interactively and re-entered for confirmation.

All subcommands operate on the server-local user store.

## Subcommands

    add       Add a new user account
    remove    Remove a user account
    list      List all user accounts
    passwd    Change a user's password

## Flags

    --json    Output as JSON (where applicable)

## user add

Add a new user account.

    zag user add -u <USERNAME> --home-dir <PATH> [--password <PASSWORD>]

Flags:

    -u, --username <USERNAME>    Username (required)
        --home-dir <PATH>        Home directory the user is locked to (required)
        --password <PASSWORD>    Password (prompted interactively if omitted)

If the home directory does not exist, it is created. A per-user log directory is also provisioned under `~/.zag/logs/users/<username>/`.

## user remove

Remove a user account.

    zag user remove <USERNAME>

Removes the user from the store. Files inside the user's home directory are left intact.

## user list

List all user accounts.

    zag user list [--json]

Prints username, home directory, and creation time for each user.

## user passwd

Change a user's password.

    zag user passwd <USERNAME> [--password <PASSWORD>]

Flags:

    --password <PASSWORD>    New password (prompted interactively if omitted)

## Examples

    # Add a user with an interactive password prompt
    zag user add -u alice --home-dir /srv/zag/alice

    # Add a user with a provided password
    zag user add -u bob --home-dir /srv/zag/bob --password secret

    # List all users
    zag user list --json

    # Change a password
    zag user passwd alice

    # Remove a user
    zag user remove bob

## See Also

    zag man serve      Run the zag HTTPS/WebSocket server
    zag man connect    Connect to a remote server (supports --username / --password)
    zag man zag        Global flags and commands
