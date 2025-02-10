import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [ver, setVer] = useState("0.0.1"); // App version
  const [mcVersions, setMcVersions] = useState<string[]>([]); // Available MC versions
  const [selectedMcVersion, setSelectedMcVersion] = useState<string>(""); // User-selected MC version
  const [forgeVer, setForgeVer] = useState<Record<string, string>>({}); // Raw Forge versions
  const [parsedForgeVer, setParsedForgeVer] = useState<Record<string, string>>({}); // Processed Forge versions
  const [fabricVer, setFabricVer] = useState<string[]>([]); // Fabric versions
  const [selectedMod, setSelectedMod] = useState<"forge" | "fabric">("forge"); // Mod type
  const [versions, setVersions] = useState<string[]>([]); // Displayed versions based on selection
  const [selectedModVersion, setSelectedModVersion] = useState<string>(""); // Selected mod version

  // Fetch app version & MC versions on mount
  useEffect(() => {
    console.log("Fetching app version...");
    invoke<string>("get_version")
      .then((version) => {
        console.log("App Version:", version);
        setVer(version);
      })
      .catch((error) => console.error("Error fetching app version:", error));

    fetchMcVersions();
  }, []);

  // Fetch Minecraft versions
  function fetchMcVersions() {
    console.log("Fetching Minecraft versions...");
    invoke<string[]>("get_minecraft_versions")
      .then((mc_versions) => {
        console.log("Minecraft Versions:", mc_versions);
        setMcVersions(Array.isArray(mc_versions) ? mc_versions : []);
      })
      .catch((error) => console.error("Error fetching Minecraft versions:", error));
  }

  // Fetch Forge versions
  function fetchForgeVersions() {
    console.log("Fetching Forge versions...");
    invoke<Record<string, string>>("get_forge_versions")
      .then((forge_versions) => {
        console.log("Raw Forge Versions:", forge_versions);
        setForgeVer(forge_versions);
      })
      .catch((error) => console.error("Error fetching Forge versions:", error));
  }

  // Process Forge versions into a clean format
  useEffect(() => {
    console.log("Processing Forge versions...");
    const parsed: Record<string, string> = {};

    Object.entries(forgeVer).forEach(([key, version]) => {
      const mcVersion = key.replace("-latest", "").replace("-recommended", "");
      if (!parsed[mcVersion] || key.endsWith("-latest")) {
        parsed[mcVersion] = version; // Prioritize latest versions
      }
    });

    console.log("Processed Forge Versions:", parsed);
    setParsedForgeVer(parsed);
  }, [forgeVer]);

  // Fetch Fabric versions
  function fetchFabricVersions() {
    console.log("Fetching Fabric versions...");
    invoke<string[]>("get_fabric_versions")
      .then((fabric_versions) => {
        console.log("Fabric Versions:", fabric_versions);
        setFabricVer(Array.isArray(fabric_versions) ? fabric_versions : []);
      })
      .catch((error) => console.error("Error fetching Fabric versions:", error));
  }

  // Fetch Forge & Fabric versions when the component mounts
  useEffect(() => {
    fetchForgeVersions();
    fetchFabricVersions();
  }, []);

  // Update available versions based on the selected mod and MC version
  useEffect(() => {
    console.log(`Mod Selection Changed: ${selectedMod}`);
    console.log(`Selected MC Version: ${selectedMcVersion}`);

    if (selectedMod === "forge" && selectedMcVersion in parsedForgeVer) {
      console.log(`Available Forge Version: ${parsedForgeVer[selectedMcVersion]}`);
      setVersions([parsedForgeVer[selectedMcVersion]]);
      setSelectedModVersion(parsedForgeVer[selectedMcVersion]); // Auto-select version
    } else if (selectedMod === "fabric") {
      console.log(`Available Fabric Versions:`, fabricVer);
      setVersions(fabricVer);
      setSelectedModVersion(fabricVer[0] || ""); // Auto-select first available
    } else {
      console.log("No available versions found.");
      setVersions([]);
      setSelectedModVersion("");
    }
  }, [selectedMod, selectedMcVersion, parsedForgeVer, fabricVer]);

  // Submit function to invoke Rust method
  function handleSubmit(event: React.FormEvent) {
    event.preventDefault();

    if (!selectedMcVersion) {
      alert("Please select a Minecraft version.");
      return;
    }
    if (!selectedModVersion) {
      alert("Please select a mod version.");
      return;
    }

    console.log("Submitting selection...");
    invoke("start_downloading", {
      minecraftVersion: selectedMcVersion,
      modTypeStr: selectedMod,
      modVersion: selectedModVersion,
    })
      .then(() => {
        console.log("Mod installation started!");
        alert("Mod installation started!");
      })
      .catch((error) => {
        console.error("Error installing mod:", error);
        alert("Failed to start installation.");
      });
  }

  return (
    <main className="container">
      <h1>Welcome to Canada Downloader</h1>

      <form onSubmit={handleSubmit}>
        {/* Select Minecraft Version */}
        <select
          value={selectedMcVersion}
          onChange={(e) => {
            console.log(`User selected Minecraft version: ${e.target.value}`);
            setSelectedMcVersion(e.target.value);
          }}
        >
          <option value="">Select Minecraft Version</option>
          {mcVersions.map((version) => (
            <option key={version} value={version}>
              {version}
            </option>
          ))}
        </select>

        {/* Select Mod Type */}
        <select
          value={selectedMod}
          onChange={(e) => {
            console.log(`User selected mod type: ${e.target.value}`);
            setSelectedMod(e.target.value as "forge" | "fabric");
          }}
        >
          <option value="forge">Forge</option>
          <option value="fabric">Fabric</option>
        </select>

        {/* Select Mod Version */}
        <select
          value={selectedModVersion}
          onChange={(e) => setSelectedModVersion(e.target.value)}
          disabled={versions.length === 0}
        >
          <option value="">Select Mod Version</option>
          {versions.map((version) => (
            <option key={version} value={version}>
              {version}
            </option>
          ))}
        </select>

        {/* Submit Button */}
        <button type="submit" disabled={!selectedMcVersion || !selectedModVersion}>
          Iniciar
        </button>
      </form>

      <h1>Version v{ver}</h1>
    </main>
  );
}

export default App;
