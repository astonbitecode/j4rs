/*
 * Copyright 2026 astonbitecode
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

import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.spy;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import java.io.File;
import java.io.IOException;
import java.util.List;
import java.util.Random;
import org.junit.Ignore;
import org.junit.Test;

public class MavenDeployerTest {

    @Ignore
    public void doDeployCallsFromApi() throws Exception {
        MavenDeployer mdspy = spy(new MavenDeployer(getRandomTmpDeployTarget()));
        mdspy.deploy("org.openjfx", "javafx-graphics", "21.0.9", "", "jar");
        verify(mdspy, times(12)).callSimpleMavenDeployer(any(), any(), any(), any(), any(), any());
    }

    @Test
    public void doDeployCallsFromApiForPomType() throws Exception {
        MavenDeployer mdspy = spy(new MavenDeployer(getRandomTmpDeployTarget()));
        mdspy.deploy("org.openjfx", "javafx-graphics", "21.0.9", "", "pom");
        verify(mdspy, times(2)).callSimpleMavenDeployer(any(), any(), any(), any(), any(), any());
    }

    @Test
    public void doDeployCallsUsesAllTheDefinedDeployers() throws Exception {
        MavenDeployer mdspy = spy(new MavenDeployer(getRandomTmpDeployTarget()));

        SimpleMavenDeployer md1 = mock(SimpleMavenDeployer.class);
        doThrow(IOException.class).when(md1).deploy(any(), any(), any(), any(), any());
        SimpleMavenDeployer md2 = mock(SimpleMavenDeployer.class);
        doThrow(IOException.class).when(md2).deploy(any(), any(), any(), any(), any());

        mdspy.callSimpleMavenDeployer("org.openjfx", "javafx-graphics", "21.0.9", "", "jar", List.of(md1, md2));
        verify(mdspy, times(2)).callSimpleMavenDeployer(any(), any(), any(), any(), any(), any());
    }

    private String getRandomTmpDeployTarget() {
        return System.getProperty("java.io.tmpdir") + File.separator + "j4rs-" + new Random().nextLong();
    }
}
