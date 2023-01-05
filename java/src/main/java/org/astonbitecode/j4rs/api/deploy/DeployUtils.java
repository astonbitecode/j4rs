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

import java.io.File;
import java.net.MalformedURLException;

public class DeployUtils {
    static void addToClasspath(String fullJarPath) throws MalformedURLException {
        if (ClassLoader.getSystemClassLoader().getClass().isAssignableFrom(J4rsClassLoader.class)) {
            J4rsClassLoader classLoader = (J4rsClassLoader) ClassLoader.getSystemClassLoader();
            File jar = new File(fullJarPath);
            classLoader.add(jar.toURI().toURL());
        }
    }
}
