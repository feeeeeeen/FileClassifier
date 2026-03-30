import { useState } from "react";
import MainWindow from "./components/MainWindow";
import DictionaryEditor from "./components/DictionaryEditor";
import LogViewer from "./components/LogViewer";

type Screen = "main" | "dictionary" | "log";

function App() {
  const [screen, setScreen] = useState<Screen>("main");

  switch (screen) {
    case "dictionary":
      return <DictionaryEditor onBack={() => setScreen("main")} />;
    case "log":
      return <LogViewer onBack={() => setScreen("main")} />;
    default:
      return (
        <MainWindow
          onOpenDictionary={() => setScreen("dictionary")}
          onOpenLog={() => setScreen("log")}
        />
      );
  }
}

export default App;
