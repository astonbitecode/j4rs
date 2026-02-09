/*
 * Copyright 2019 astonbitecode
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

import org.junit.Test;

import java.io.File;
import java.io.FileNotFoundException;

public class FileSystemDeployerTest {
    @Test
    public void deploySuccess() throws Exception {
        // Download a jar file first
        SimpleMavenDeployer md = new SimpleMavenDeployer();

        md.deploy("io.github.astonbitecode", "j4rs", "0.5.1", "");

        FileSystemDeployer fsd = new FileSystemDeployer("./fsdTarget");

        fsd.deploy("./j4rs-0.5.1.jar");

        File f1 = new File("./j4rs-0.5.1.jar");
        File f2 = new File("./fsdTarget/j4rs-0.5.1.jar");
        File f3 = new File("./fsdTarget");
        f1.delete();
        f2.delete();
        f3.delete();
    }

    @Test(expected = FileNotFoundException.class)
    public void deployFailure() throws Exception {
        FileSystemDeployer fsd = new FileSystemDeployer();

        fsd.deploy("./NonExistingJar.jar");
    }

}
