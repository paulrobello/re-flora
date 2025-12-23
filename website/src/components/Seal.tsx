import React, { useState } from 'react';

export default function Seal({ onOpen }: { onOpen: () => void }) {
  const [isOpen, setIsOpen] = useState(false);
  const [isBroken, setIsBroken] = useState(false);

  const handleOpen = () => {
    setIsBroken(true);
    setTimeout(() => {
      setIsOpen(true);
      setTimeout(onOpen, 1000); // Allow transition to finish before unmounting or doing anything else
    }, 600);
  };

  return (
    <div className={`envelope-overlay ${isOpen ? 'open' : ''}`}>
      <div 
        className={`wax-seal ${isBroken ? 'broken' : ''}`} 
        onClick={handleOpen}
        role="button"
        aria-label="Open Letter"
      >
      </div>
      {!isBroken && (
        <div className="absolute mt-40 text-stone-300 font-serif italic text-lg opacity-80 animate-pulse pointer-events-none">
          Click the seal to open
        </div>
      )}
    </div>
  );
}
