/*
 * Copyright 2026 astonbitecode Licensed under the Apache License, Version 2.0 (the "License"); you
 * may not use this file except in compliance with the License. You may obtain a copy of the License
 * at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software distributed under the License
 * is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
 * or implied. See the License for the specific language governing permissions and limitations under
 * the License.
 */
package org.astonbitecode.j4rs.api.deploy;

import java.io.File;
import java.io.IOException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.apache.maven.model.Dependency;
import org.apache.maven.model.Model;
import org.apache.maven.model.Repository;
import org.apache.maven.model.building.DefaultModelBuilderFactory;
import org.apache.maven.model.building.DefaultModelBuildingRequest;
import org.apache.maven.model.building.FileModelSource;
import org.apache.maven.model.building.ModelBuilder;
import org.apache.maven.model.building.ModelBuildingResult;
import org.apache.maven.model.building.ModelSource;
import org.apache.maven.model.resolution.InvalidRepositoryException;
import org.apache.maven.model.resolution.ModelResolver;
import org.apache.maven.model.resolution.UnresolvableModelException;

public class MavenDeployer implements MavenDeployerApi {
    private static final String POM_TYPE = "pom";
    private final SimpleMavenDeployer simpleMavenDeployer;
    private final Map<String, SimpleMavenDeployer> additionalDeployers = new HashMap<>();

    public MavenDeployer() {
        this.simpleMavenDeployer = new SimpleMavenDeployer();
    }

    public MavenDeployer(String deployTarget) {
        this.simpleMavenDeployer = new SimpleMavenDeployer(deployTarget);
    }

    public MavenDeployer(String repoBase, String deployTarget) {
        this.simpleMavenDeployer = new SimpleMavenDeployer(repoBase, deployTarget);
    }

    public MavenDeployer(String repoBase, boolean checkLocalCache, String deployTarget) {
        this.simpleMavenDeployer = new SimpleMavenDeployer(repoBase, checkLocalCache, deployTarget);
    }

    @Override
    public void deploy(String groupId, String artifactId, String version, String qualifier) throws IOException {
        this.deploy(groupId, artifactId, version, qualifier, "jar");
    }

    @Override
    public void deploy(String groupId, String artifactId, String version, String qualifier, String artifactType) throws IOException {
        if (!DeployUtils.artifactExists(groupId, artifactId, version, qualifier, artifactType, artifactType)) {
            doDeploy(groupId, artifactId, version, qualifier, artifactType, gatherAllDeployers());
            if (!artifactType.equals(POM_TYPE)) {
                // For pom types the qualifiers should not be defined
                doDeploy(groupId, artifactId, version, "", POM_TYPE, gatherAllDeployers());
            }
            // For pom types the qualifiers should not be defined
            String pomArtifactName = DeployUtils.generateArtifactName(artifactId, version, "", POM_TYPE);
            String pathString = getDeployTarget() + File.separator + pomArtifactName;
            File pomFile = new File(pathString);
            try {
                List<Dependency> dependencies = parsePom(pomFile);
                if (dependencies != null) {
                    for (Dependency dep : dependencies) {
                        if (!dep.getArtifactId().contains("j4rs") && dep.getType().equals(artifactType) && !dep.getScope().equals("test") && !dep.getScope().equals("provided")) {
                            // Deploy only for the needed os
                            if (dep.getClassifier() == null || dep.getClassifier().length() == 0 || dep.getClassifier().contains(System.getProperty("os.name").toLowerCase())) {
                                deploy(dep.getGroupId(), dep.getArtifactId(), dep.getVersion(), dep.getClassifier(), artifactType);
                            }
                        }
                    }
                }
            } catch (Exception error) {
                error.printStackTrace();
                throw new IOException(error);
            } finally {
                if (!POM_TYPE.equals(artifactType)) {
                    pomFile.delete();
                }
            }
        }
    }

    List<Dependency> parsePom(File pomFile) throws Exception {
        final DefaultModelBuildingRequest modelBuildingRequest = new DefaultModelBuildingRequest().setPomFile(pomFile);
        modelBuildingRequest.setModelResolver(new J4rsMavenModelResolver());
        modelBuildingRequest.setSystemProperties(System.getProperties());
        ModelBuilder modelBuilder = new DefaultModelBuilderFactory().newInstance();
        ModelBuildingResult modelBuildingResult = modelBuilder.build(modelBuildingRequest);

        Model model = modelBuildingResult.getEffectiveModel();
        List<Dependency> deps = model.getDependencies();
        return deps;
    }

    public String getRepoBase() {
        return simpleMavenDeployer.getRepoBase();
    }

    public String getDeployTarget() {
        return simpleMavenDeployer.getDeployTarget();
    }

    private List<SimpleMavenDeployer> gatherAllDeployers() {
        List<SimpleMavenDeployer> deployers = new ArrayList<>();
        deployers.add(simpleMavenDeployer);
        deployers.addAll(additionalDeployers.values());
        return deployers;
    }

    void doDeploy(String groupId, String artifactId, String version, String qualifier, String artifactType, List<SimpleMavenDeployer> deployers) throws IOException {
        if (deployers != null && deployers.size() > 0) {
            try {
                deployers.get(0).deploy(groupId, artifactId, version, qualifier, artifactType);
            } catch (IOException error) {
                if (deployers.size() == 1) {
                    throw error;
                } else {
                    List<SimpleMavenDeployer> reducedDeployers = deployers.subList(1, deployers.size() - 1);
                    doDeploy(groupId, artifactId, version, qualifier, artifactType, reducedDeployers);
                }
            }
        }
    }

    private class J4rsMavenModelResolver implements ModelResolver {

        @Override
        public ModelSource resolveModel(String groupId, String artifactId, String version) throws UnresolvableModelException {

            try {
                doDeploy(groupId, artifactId, version, "", POM_TYPE, gatherAllDeployers());
                String pomArtifactName = DeployUtils.generateArtifactName(artifactId, version, "", POM_TYPE);
                String pathString = getDeployTarget() + File.separator + pomArtifactName;
                return new FileModelSource(new File(pathString));
            } catch (IOException error) {
                throw new UnresolvableModelException("Could not resolve model", groupId, artifactId, version, error);
            }
        }

        @Override
        public void addRepository(Repository repository) throws InvalidRepositoryException {
            additionalDeployers.put(repository.getUrl(), new SimpleMavenDeployer(getRepoBase(), repository.getUrl()));
        }

        @Override
        public ModelResolver newCopy() {
            return new J4rsMavenModelResolver();
        }

    }
}
