// Copyright 2011-2023 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package com.google.security.zynamics.zylib.net;

public class NetHelpers {
  private static final int MAX_PORT = 65535;

  public static boolean isValidPort(final int port) {
    return (port >= 0) && (port <= MAX_PORT);
  }

  public static boolean isValidPort(final String port) {
    try {
      Integer.parseInt(port);
      return true;
    } catch (final NumberFormatException e) {
      return false;
    }
  }
}
