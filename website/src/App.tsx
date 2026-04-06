import { Routes, Route } from "react-router-dom";
import Navbar from "./components/Navbar";
import Hero from "./components/Hero";
import Features from "./components/Features";
import Providers from "./components/Providers";
import Orchestration from "./components/Orchestration";
import CodeExamples from "./components/CodeExamples";
import Bindings from "./components/Bindings";
import GettingStarted from "./components/GettingStarted";
import Footer from "./components/Footer";
import Documentation from "./components/Documentation";
import Manual from "./components/Manual";

function LandingPage() {
  return (
    <>
      <Hero />
      <Features />
      <Providers />
      <Orchestration />
      <CodeExamples />
      <Bindings />
      <GettingStarted />
    </>
  );
}

export default function App() {
  return (
    <div className="min-h-screen bg-surface overflow-x-hidden">
      <Navbar />
      <Routes>
        <Route path="/" element={<LandingPage />} />
        <Route path="/docs" element={<Documentation />} />
        <Route path="/docs/:slug" element={<Documentation />} />
        <Route path="/manual" element={<Manual />} />
        <Route path="/manual/:slug" element={<Manual />} />
      </Routes>
      <Footer />
    </div>
  );
}
