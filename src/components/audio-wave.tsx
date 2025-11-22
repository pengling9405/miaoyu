import { motion } from "motion/react";
import { memo, useEffect, useState } from "react";

export const AudioWave = memo(() => {
  const [recordingBars, setRecordingBars] = useState<number[]>(
    Array(12).fill(5),
  );
  useEffect(() => {
    const interval = setInterval(() => {
      setRecordingBars((bars) =>
        bars.map(() => Math.floor(Math.random() * 10) + 3),
      );
    }, 200);

    return () => {
      clearInterval(interval);
    };
  }, []);
  return (
    <div className="flex gap-0.5 justify-center items-center">
      {recordingBars.map((height, i) => (
        <motion.div
          key={`bar-${i}`}
          animate={{ height: `${height}px` }}
          transition={{ duration: 0.2, ease: "easeInOut" }}
          className="w-0.5 bg-foreground rounded-full"
        />
      ))}
    </div>
  );
});

AudioWave.displayName = "AudioWave";
