#!/bin/sh

cleanup() {
  sudo cpupower -c $N frequency-set --governor "$POLICY"
  if [ $? -ne 0 ]; then
    echo "error: failed to revert policy; manual intervention is required" >&2
    echo "note: previous policy was '$POLICY'" >&2
    exit 6
  fi
}

cpupower --version > /dev/null
if [ $? -ne 0 ]; then
  echo "error: cpupower not installed" >&2
  exit 1
fi

cargo --version > /dev/null
if [ $? -ne 0 ]; then
  echo "error: cargo not installed" >&2
  exit 2
fi

N=$(ls /sys/devices/system/cpu | grep -Eo '[0-9]+' | sort -h | tail -1)
if [ $? -ne 0 ]; then
  echo "error: failed to retrieve cpu id" >&2
  exit 3
fi

POLICY=$(cat "/sys/devices/system/cpu/cpu$N/cpufreq/scaling_governor")
if [ $? -ne 0 ]; then
  echo "error: failed to get policy" >&2
  exit 4
fi

sudo cpupower -c $N frequency-set --governor performance
if [ $? -ne 0 ]; then
  echo "error: failed to set policy" >&2
  exit 5
fi

trap cleanup EXIT INT TERM

taskset -c $N cargo bench --all-features
