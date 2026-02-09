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
import java.io.IOException;

import static org.mockito.Mockito.*;

public class SimpleMavenDeployerTest {
    @Test
    public void repoBase() {
        assert (new SimpleMavenDeployer("my", true, "depltarget").getRepoBase().equals("my"));
        File f = new File("depltarget");
        f.delete();
    }

    @Test
    public void generateUrlTagret() throws IOException {
        assert (new SimpleMavenDeployer("https://my.artifactory.com", true, "depltarget")
                .generateUrlTagret("io.github.astonbitecode", "j4rs", "0.5.1", "j4rs-0.5.1.jar")
                .equals("https://my.artifactory.com/io/github/astonbitecode/j4rs/0.5.1/j4rs-0.5.1.jar"));

        File f = new File("depltarget");
        f.delete();
    }

    @Test
    public void deploySuccess() throws Exception {
        SimpleMavenDeployer md = new SimpleMavenDeployer();

        md.deploy("io.github.astonbitecode", "j4rs", "0.5.1", "");

        File f = new File("./j4rs-0.5.1.jar");
        f.delete();
    }

    @Test(expected = IOException.class)
    public void deployFailure() throws Exception {
        SimpleMavenDeployer md = new SimpleMavenDeployer();

        md.deploy("io.github.astonbitecode", "j4rs", "non-existing", "");
    }

    @Test()
    public void doNotDownloadArtifactIfAlreadyDeployed() throws Exception {
        new SimpleMavenDeployer().deploy("io.github.astonbitecode", "j4rs", "0.5.1", "");

        SimpleMavenDeployer mdmock = mock(SimpleMavenDeployer.class);
        mdmock.deploy("io.github.astonbitecode", "j4rs", "0.5.1", "");

        verify(mdmock, times(0)).deployFromLocalCache(any(), any(), any(), any(), any());

        File f = new File("./j4rs-0.5.1.jar");
        f.delete();
    }

}
