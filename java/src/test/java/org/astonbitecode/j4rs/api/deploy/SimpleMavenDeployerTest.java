package org.astonbitecode.j4rs.api.deploy;

import org.junit.Test;

import java.io.File;
import java.io.FileNotFoundException;

public class SimpleMavenDeployerTest {
    @Test
    public void repoBase() {
        assert (new SimpleMavenDeployer("my", true, "depltarget").getRepoBase().equals("my"));
    }

    @Test
    public void generateArtifactName() {
        assert (new SimpleMavenDeployer().generateArtifactName(
                "j4rs",
                "0.5.1",
                "").equals("j4rs-0.5.1.jar"));
    }

    @Test
    public void generateUrlTagret() {
        assert (new SimpleMavenDeployer("https://my.artifactory.com", true, "depltarget").generateUrlTagret(
                "io.github.astonbitecode",
                "j4rs",
                "0.5.1",
                "j4rs-0.5.1.jar").equals("https://my.artifactory.com/io/github/astonbitecode/j4rs/0.5.1/j4rs-0.5.1.jar"));
    }

    @Test
    public void deploySuccess() throws Exception {
        SimpleMavenDeployer md = new SimpleMavenDeployer();

        md.deploy("io.github.astonbitecode",
                "j4rs",
                "0.5.1",
                "");

        File f = new File("./j4rs-0.5.1.jar");
        f.delete();
    }

    @Test(expected = FileNotFoundException.class)
    public void deployFailure() throws Exception {
        SimpleMavenDeployer md = new SimpleMavenDeployer();

        md.deploy("io.github.astonbitecode",
                "j4rs",
                "non-existing",
                "");
    }
}
