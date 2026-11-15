declare module "mapshaper" {
  const mapshaper: {
    applyCommands(
      commands: string,
      input: Record<string, Buffer | string>,
    ): Promise<Record<string, Buffer | string>>;
  };
  export default mapshaper;
}
