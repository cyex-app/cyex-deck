// V1051-01 Chromium knows speakers by ids of its own; the app stores Windows' names.

export interface MediaDevice {
  kind: string;
  deviceId: string;
  label: string;
}

/** Chromium's label as Windows names the device: without the "(1234:abcd)" USB id it adds. */
export function windowsName(label: string): string {
  return label.replace(/ \([0-9a-f]{4}:[0-9a-f]{4}\)$/i, '').trim();
}

/** Chromium's id of the output Windows calls `name`; '' (the default) when it isn't there. */
export function sinkFor(devices: readonly MediaDevice[], name: string): string {
  const found = devices.find(
    (d) =>
      d.kind === 'audiooutput' &&
      d.deviceId !== 'default' &&
      d.deviceId !== 'communications' &&
      windowsName(d.label) === name,
  );
  return found?.deviceId ?? '';
}
