import Config from '../utils/Config';

export default async ({}) => {
  let configPath;
  if (process.env.DEV) {
    configPath = '/headway-dev-config.json';
  } else {
    configPath = '/static/headway-config.json';
  }
  console.debug('using config', configPath);
  let response = await fetch(configPath);

  let json = await response.json();
  Config.shared = json;
};
