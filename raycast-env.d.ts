/// <reference types="@raycast/api">

/* 🚧 🚧 🚧
 * This file is auto-generated from the extension's manifest.
 * Do not modify manually. Instead, update the `package.json` file.
 * 🚧 🚧 🚧 */

/* eslint-disable @typescript-eslint/ban-types */

type ExtensionPreferences = {
  /** undefined - Close extension on action execution (Copy to Clipboard / Open in Browser) */
  "closeOnAction": boolean
}

/** Preferences accessible in all the extension's commands */
declare type Preferences = ExtensionPreferences

declare namespace Preferences {
  /** Preferences accessible in the `encode-coordinates` command */
  export type EncodeCoordinates = ExtensionPreferences & {}
  /** Preferences accessible in the `decode-geohash` command */
  export type DecodeGeohash = ExtensionPreferences & {}
}

declare namespace Arguments {
  /** Arguments passed to the `encode-coordinates` command */
  export type EncodeCoordinates = {}
  /** Arguments passed to the `decode-geohash` command */
  export type DecodeGeohash = {}
}

