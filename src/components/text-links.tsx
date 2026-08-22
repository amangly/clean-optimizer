type Props = {
  text: string;
  className?: string;
};

export function TextLinks({ text, className }: Props) {
  const parts = text.split(/(https:\/\/[^\s]+)/g);
  return (
    <span className={className}>
      {parts.map((part, i) =>
        part.startsWith("https://") ? (
          <a key={`${part}-${i}`} href={part} className="underline underline-offset-2">
            {part}
          </a>
        ) : (
          <span key={`${part}-${i}`}>{part}</span>
        ),
      )}
    </span>
  );
}
