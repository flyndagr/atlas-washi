const views = {
 notebook: ['Slow down. Notice what matters.', 'A page that feels like paper. Notes saved as Markdown on your Mac.', 'Atlas notebook with fountain-pen lettering and sample Markdown notes'],
 canvas: ['One thought leads to another.', 'Arrange notes in space. Their wiki links become the threads between them.', 'Atlas canvas with connected sample note cards'],
 remember: ['Keep the thought. Keep the context.', 'A source, a person, a review date. Return when you’re ready.', 'Atlas Remember this review showing fictional example reminders']
};
const tabs = [...document.querySelectorAll('[role="tab"]')];
function select(tab) {
 tabs.forEach(t => {t.setAttribute('aria-selected', String(t === tab)); t.tabIndex = t === tab ? 0 : -1;});
 const key = tab.dataset.view, [title, copy, alt] = views[key];
 const image = document.getElementById('app-preview'); image.src = `assets/${key}.webp`; image.alt = alt;
 document.getElementById('preview-title').textContent = title;
 document.getElementById('preview-copy').textContent = copy;
 document.getElementById('preview').setAttribute('aria-labelledby', tab.id);
}
tabs.forEach((tab, i) => {
 tab.addEventListener('click', () => select(tab));
 tab.addEventListener('keydown', event => {
  let next;
  if(event.key === 'ArrowRight') next = (i+1)%tabs.length;
  if(event.key === 'ArrowLeft') next = (i+tabs.length-1)%tabs.length;
  if(event.key === 'Home') next = 0;
  if(event.key === 'End') next = tabs.length-1;
  if(next !== undefined){event.preventDefault();select(tabs[next]);tabs[next].focus();}
 });
});
