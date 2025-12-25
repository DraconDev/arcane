#!/bin/bash

# Sanitize environment to fix VS Code Snap crashes
# We use 'env -u' to verify we are actually removing them from the child process

# List of variables to unset
VARS_TO_UNSET=(
    "GTK_PATH"
    "GTK_EXE_PREFIX"
    "GTK_MODULES"
    "GTK_IM_MODULE_FILE"
    "GDK_BACKEND"
    "GIO_MODULE_DIR"
    "LD_LIBRARY_PATH"
)

# Build the command
CMD="env"
for var in "${VARS_TO_UNSET[@]}"; do
    CMD="$CMD -u $var"
done

# Add required fix for WebKit crash
CMD="$CMD WEBKIT_DISABLE_DMABUF_RENDERER=1 GDK_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1 RUST_BACKTRACE=1 RUST_LOG=info"

# Force clean XDG_DATA_DIRS to avoid loading Snap schemas
CMD="$CMD XDG_DATA_DIRS=/usr/local/share:/usr/share"

# Execute the target command
exec $CMD "$@"
