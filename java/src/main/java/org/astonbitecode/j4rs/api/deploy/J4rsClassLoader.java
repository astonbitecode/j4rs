/*
 * Copyright 2023 astonbitecode
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
package org.astonbitecode.j4rs.api.deploy;

import java.net.URL;
import java.net.URLClassLoader;

public final class J4rsClassLoader extends URLClassLoader {
    public J4rsClassLoader(ClassLoader offeredClassLoader) {
        super("j4rsClassLoader", new URL[0], offeredClassLoader);
    }

    void add(URL url) {
        addURL(url);
    }
}
