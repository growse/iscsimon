# iscsimon Requirements

iscsimon is a CLI / TUI tool that can allow a user to monitor the current iscsi connections to an iscsi Linux target host.

## Features

- The list of currently open iscsi connections to the target should be displayed
- For each entry, I should be able to see the source of the connection (similar to what we get with netstat)
- I should also be able to see which iscsi target (by name) is being used on that connection, and the associated block storage device
- I should see TX/RX rates for that connection too
- The TUI should be beautiful and easy to use with keyboard shortcuts.
