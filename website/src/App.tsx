import Navbar from "./components/Navbar";
import Hero from "./components/Hero";
import Features from "./components/Features";
import Providers from "./components/Providers";
import Orchestration from "./components/Orchestration";
import CodeExamples from "./components/CodeExamples";
import Bindings from "./components/Bindings";
import GettingStarted from "./components/GettingStarted";
import Footer from "./components/Footer";

export default function App() {
  return (
    <div className="min-h-screen bg-surface">
      <Navbar />
      <Hero />
      <Features />
      <Providers />
      <Orchestration />
      <CodeExamples />
      <Bindings />
      <GettingStarted />
      <Footer />
    </div>
  );
}
