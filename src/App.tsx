import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

function App() {
  const [ver, setVer] = useState("0.0.1");
  const [mcVersions, setMcVersions] = useState<string[]>([]);
  const [selectedMcVersion, setSelectedMcVersion] = useState<string>("");
  const [forgeVer, setForgeVer] = useState<Record<string, string>>({});
  const [parsedForgeVer, setParsedForgeVer] = useState<Record<string, string>>({});
  const [fabricVer, setFabricVer] = useState<string[]>([]);
  const [selectedMod, setSelectedMod] = useState<"forge" | "fabric">("forge");
  const [versions, setVersions] = useState<string[]>([]);
  const [selectedModVersion, setSelectedModVersion] = useState<string>("");
  const [logs, setLogs] = useState<string[]>([]);
  const [isDownloading, setIsDownloading] = useState(false);

  const logContainerRef = useRef<HTMLDivElement>(null);

  // Set random background
  const wallpapers = [
    "https://wallpapercave.com/wp/wp14014986.webp",
    "https://wallpapercave.com/wp/wp14924716.webp",
    "https://wallpapercave.com/wp/wp14924717.webp",
    "https://wallpapercave.com/wp/wp14924718.webp",
    "https://wallpapercave.com/wp/wp14912676.webp",
    "https://wallpapercave.com/wp/wp14924724.webp",
    "https://wallpapercave.com/wp/wp14924726.webp",
    "https://wallpapercave.com/wp/wp14924728.webp",
    "https://wallpapercave.com/wp/wp14924732.webp",
    "https://wallpapercave.com/wp/wp14924737.webp",
    "https://wallpapercave.com/wp/wp14924738.webp",
    "https://wallpapercave.com/wp/wp14924740.webp",
    "https://wallpapercave.com/wp/wp14924742.jpg",
    "https://wallpapercave.com/wp/wp14924744.webp",
  ];

  useEffect(() => {
    fetchVersion();
    fetchMcVersions();
    fetchFabricVersions();
    fetchForgeVersions();
    get_saved_data();
    const randomImage = wallpapers[Math.floor(Math.random() * wallpapers.length)];
    document.body.style.background = `url(${randomImage}) no-repeat center center fixed`;
    document.body.style.backgroundSize = "cover";
  }, []);

  interface LogEventPayload {
    message: string;
  }

  interface Data {
    minecraft_version: string;
    mod_loader: string;
    mod_loader_version: string;
  }
  
  function get_saved_data() {
    invoke<Data>("get_data")
      .then((data) => {
        if (data) {
          setSelectedMcVersion(data.minecraft_version);
          setSelectedMod(data.mod_loader as "forge" | "fabric");
          setSelectedModVersion(data.mod_loader_version);
        }
      })
      .catch((error) => console.error("Error fetching saved data:", error));
  }
  
  useEffect(() => {
    // Start listening for log events
    const unsubscribe = listen("log-event", (event: { payload: LogEventPayload }) => {
      const message = event.payload.message;
      setLogs((prevLogs) => [...prevLogs, message]);
    });

    // Cleanup listener on component unmount
    return () => {
      unsubscribe.then((unsub) => unsub());
    };
  }, []);

  useEffect(() => {
    if (logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
    }
  }, [logs]);

  function fetchVersion() {
    invoke<string>("get_version").then((version) => setVer(version));
  }

  function fetchMcVersions() {
    invoke<string[]>("get_minecraft_versions")
      .then((mc_versions) => setMcVersions(Array.isArray(mc_versions) ? mc_versions : []))
      .catch((error) => console.error("Error al obtener versiones de Minecraft:", error));
  }

  function fetchForgeVersions() {
    invoke<Record<string, string>>("get_forge_versions")
      .then((forge_versions) => setForgeVer(forge_versions))
      .catch((error) => console.error("Error al obtener versiones de Forge:", error));
  }

  function fetchFabricVersions() {
    invoke<string[]>("get_fabric_versions")
      .then((fabric_versions) => setFabricVer(Array.isArray(fabric_versions) ? fabric_versions : []))
      .catch((error) => console.error("Error al obtener versiones de Fabric:", error));
  }

  useEffect(() => {
    const parsed: Record<string, string> = {};
    Object.entries(forgeVer).forEach(([key, version]) => {
      const mcVersion = key.replace("-latest", "").replace("-recommended", "");
      if (!parsed[mcVersion] || key.endsWith("-latest")) {
        parsed[mcVersion] = version;
      }
    });
    setParsedForgeVer(parsed);
  }, [forgeVer]);

  useEffect(() => {
    if (selectedMod === "forge" && selectedMcVersion in parsedForgeVer) {
      setVersions([parsedForgeVer[selectedMcVersion]]);
      setSelectedModVersion(parsedForgeVer[selectedMcVersion]);
    } else if (selectedMod === "fabric") {
      setVersions(fabricVer);
      setSelectedModVersion(fabricVer[0] || "");
    } else {
      setVersions([]);
      setSelectedModVersion("");
    }
  }, [selectedMod, selectedMcVersion, parsedForgeVer, fabricVer]);

  function handleSubmit(event: React.FormEvent) {
    invoke("save_data", {
      minecraftVersion: selectedMcVersion,
      modLoader: selectedMod,
      modLoaderVersion: selectedModVersion
    })
    event.preventDefault();
    if (!selectedMcVersion || !selectedModVersion) return;

    setIsDownloading(true);

    invoke("start_downloading", {
      minecraftVersion: selectedMcVersion,
      modTypeStr: selectedMod,
      modVersion: selectedModVersion,
    })
      .then(() => setLogs((prevLogs) => [...prevLogs, "✅ Instalación iniciada..."]))
      .catch((error) => {
        console.error("Error al instalar el mod:", error);
        setLogs((prevLogs) => [...prevLogs, "❌ Error al iniciar la instalación."]);
      })
      .finally(() => setIsDownloading(false));
  }

  return (
    <main className="container">
      <form onSubmit={handleSubmit}>
        <label>Selecciona la versión de Minecraft:</label>
        <select value={selectedMcVersion} onChange={(e) => setSelectedMcVersion(e.target.value)}>
          <option value="">Seleccionar versión de Minecraft</option>
          {mcVersions.map((version) => (
            <option key={version} value={version}>{version}</option>
          ))}
        </select>

        <label>Selecciona el Mod loader:</label>
        <select value={selectedMod} onChange={(e) => setSelectedMod(e.target.value as "forge" | "fabric")}>
          <option value="forge">Forge</option>
          <option value="fabric">Fabric</option>
        </select>

        <label>Selecciona la versión del Mod Loader:</label>
        <select value={selectedModVersion} onChange={(e) => setSelectedModVersion(e.target.value)} disabled={versions.length === 0}>
          <option value="">Seleccionar versión del mod</option>
          {versions.map((version) => (
            <option key={version} value={version}>{version}</option>
          ))}
        </select>

        <button type="submit" disabled={!selectedMcVersion || !selectedModVersion || isDownloading}>
          {isDownloading ? "Iniciando..." : "Iniciar"}
        </button>
      </form>

      <h2>Registro de instalación:</h2>
      <div className="log-box" ref={logContainerRef} style={{ maxHeight: "300px", overflowY: "auto" }}>
        {logs.length === 0 ? (
          <p>No hay registros aún.</p>
        ) : (
          logs.map((log, index) => <p key={index}>{log}</p>)
        )}
      </div>

      <h2 className="version-text">Versión v{ver}</h2>
    </main>
  );
}

export default App;
